use std::{
    borrow::BorrowMut,
    collections::HashMap,
    fmt,
    io::{Read, Write},
};

use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{
    author::Author,
    aux::*,
    blob::Blob,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
    super_string::{read_string_from, u8_vec_to_hex_string, SuperStrings},
};

#[derive(Clone)]
pub struct Tree {
    path: String,
    objects: HashMap<String, GitObject>,
    hash: Option<[u8; 20]>,
}

impl Tree {
    pub fn has_blob_from_hash(
        &mut self,
        blob_hash: &str,
        logger: &mut Logger,
    ) -> Result<(bool, String), CommandError> {
        Ok(Self::has_blob_from_hash_aux(
            &mut self.get_objects(),
            logger,
            blob_hash,
        )?)
    }

    fn has_blob_from_hash_aux(
        objects: &mut HashMap<String, GitObject>,
        logger: &mut Logger,
        blob_hash: &str,
    ) -> Result<(bool, String), CommandError> {
        for (name, object) in objects.iter_mut() {
            let hash = object.get_hash_string()?;
            logger.log(&format!(
                "Has blob from hash --> Name: {}. hash: {}",
                name, hash
            ));
            if hash == blob_hash.to_string() {
                logger.log(&format!("true"));

                return Ok((true, name.to_owned()));
            }
            if let Some(tree) = object.as_mut_tree() {
                let (found, name) =
                    Self::has_blob_from_hash_aux(&mut tree.get_objects(), logger, blob_hash)?;
                if found {
                    return Ok((true, name));
                }
            }
        }

        Ok((false, "".to_string()))
    }

    pub fn has_blob_from_path(&self, path: &str, logger: &mut Logger) -> bool {
        let mut parts: Vec<&str> = path.split_terminator("/").collect();
        return self.follow_path_in_tree(&mut parts, logger);
    }

    fn follow_path_in_tree(&self, path: &mut Vec<&str>, logger: &mut Logger) -> bool {
        if path.is_empty() {
            logger.log(&format!("Path empty"));

            return false;
        }
        for (name, object) in self.get_objects().iter_mut() {
            logger.log(&format!("Name: {}, part: {}", name, path[0]));

            if name == path[0] {
                logger.log(&format!("found"));

                if let Some(obj_tree) = object.as_tree() {
                    _ = path.remove(0);
                    return obj_tree.follow_path_in_tree(path, logger);
                }
                logger.log(&format!("return true"));

                return true;
            }
        }
        false
    }

    pub fn get_deleted_blobs_from_path(
        &mut self,
        files: Vec<&str>,
        logger: &mut Logger,
    ) -> Vec<String> {
        let mut deleted_blobs: Vec<String> = Vec::new();
        for file in files.iter() {
            logger.log(&format!("Path buscado: {}", file));
            if !self.has_blob_from_path(file, logger) {
                logger.log(&format!("No encontrado: {}", file));

                deleted_blobs.push(file.to_string());
            }
        }
        deleted_blobs
    }

    /* fn a(&self, path: &mut Vec<&str>, files: Vec<&String>) -> bool {
        if path.is_empty() {
            return false;
        }
        for (name, object) in self.get_objects().iter_mut() {
            if name == path[0] {
                if let Some(obj_tree) = object.as_tree() {
                    _ = path.remove(0);
                    return obj_tree.a(path);
                }
                return true;
            }
        }
        false
    } */

    pub fn add_path_tree(
        &mut self,
        logger: &mut Logger,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        Ok(self.add_path(logger, vector_path, current_depth, hash)?)
    }

    /// Crea un Tree vacío a partir de su ruta.
    pub fn new(path: String) -> Tree {
        Tree {
            path: path.clone(),
            objects: HashMap::new(),
            hash: None,
        }
    }

    /// Devuelve los subdirectorios o archivos que contiene el Tree (directorio).
    pub fn add_object(&mut self, name: String, object: GitObject) {
        _ = self.objects.insert(name, object);
    }

    pub fn get_objects(&self) -> HashMap<String, GitObject> {
        self.objects.clone()
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(
        &mut self,
        logger: &mut Logger,
        path_name: &String,
        hash: &String,
    ) -> Result<(), CommandError> {
        // let blob = Blob::new_from_hash(hash.clone(), path_name.clone())?;
        let blob =
            Blob::new_from_hash_and_mode(hash.clone(), path_name.clone(), Mode::RegularFile)?;
        let blob_name = get_name(&path_name)?;
        _ = self.objects.insert(blob_name.to_string(), Box::new(blob));
        Ok(())
    }

    pub fn add_tree(
        &mut self,
        logger: &mut Logger,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        let current_path = vector_path[..current_depth + 1].join("/");
        let tree_name = get_name(&current_path)?;

        if !self.objects.contains_key(&tree_name) {
            let tree = Tree::new(current_path.to_owned());
            self.objects.insert(tree_name.clone(), Box::new(tree));
        }

        let Some(tree) = self.objects.get_mut(&tree_name) else {
            return Err(CommandError::ObjectNotTree);
        };

        tree.add_path(logger, vector_path, current_depth + 1, hash)?;
        Ok(())
    }

    /* fn get_data(&mut self) -> Result<Vec<u8>, CommandError> {
        let header = format!("1 {}\0", self.size()?);
        let content = self.content()?;
        Ok([header.as_bytes(), content.as_slice()].concat())
    } */

    /* fn get_mode(&self) -> Result<Mode, CommandError> {
        Ok(Mode::get_mode(self.path.clone())?)
    } */

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        _: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut objects = HashMap::<String, GitObject>::new();

        while let Ok(mode) = read_mode(stream) {
            let name = read_string_until(stream, '\0')?;
            let mut hash = vec![0; 20];
            stream
                .read_exact(&mut hash)
                .map_err(|_| CommandError::ObjectHashNotKnown)?;
            let hash_str = u8_vec_to_hex_string(&hash);

            let object = objects_database::read_object(&hash_str, logger)?;
            objects.insert(name, object);
        }
        Ok(Box::new(Self {
            path: path.to_string(),
            objects,
            hash: None,
        }))
    }

    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        _: usize,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut objects = Vec::<(Mode, String, String, String)>::new();
        loop {
            let Ok(string_part_bytes) = read_string_until(stream, '\0') else {
                break;
            };

            let Some((mode, name)) = string_part_bytes.split_once(' ') else {
                return Err(CommandError::FileReadError(
                    "No se pudo leer objeto".to_string(),
                ));
            };
            let hash_str = get_hash(stream)?;
            let mode = get_mode(mode)?;
            let object_type = Mode::get_type_from_mode(&mode);
            objects.push((mode, object_type, hash_str, name.to_string()));
        }

        for (mode, type_str, hash, name) in objects {
            writeln!(output, "{} {} {}    {}", mode, type_str, hash, name)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
            output
                .flush()
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }

    pub fn sort_objects(&self) -> Vec<(String, GitObject)> {
        let mut names_objects: Vec<&String> = self.objects.keys().collect();
        names_objects.sort();

        let mut sorted_objects: Vec<(String, GitObject)> = Vec::new();
        for name_object in names_objects {
            if let Some(object) = self.objects.get(name_object) {
                sorted_objects.push((name_object.clone(), object.clone()));
            }
        }
        sorted_objects
    }

    pub fn set_hash(&mut self, hash: [u8; 20]) {
        self.hash = Some(hash);
    }
}

fn read_mode(stream: &mut dyn Read) -> Result<Mode, CommandError> {
    let mode_str = read_string_until(stream, ' ')?;
    let mode = Mode::read_from_string(&mode_str)?;
    Ok(mode)
}

fn get_mode(mode: &str) -> Result<Mode, CommandError> {
    let id = mode
        .parse::<u32>()
        .map_err(|_| CommandError::FileReadError("No se pudo leer objeto".to_string()))?;
    let mode = Mode::get_mode_from_id(id)
        .map_err(|_| CommandError::FileReadError("No se pudo leer objeto".to_string()))?;
    Ok(mode)
}

fn get_mode_and_name(buf: Vec<u8>) -> Result<(String, String), CommandError> {
    let string_mode_name = String::from_utf8(buf).map_err(|_| CommandError::InvalidMode)?;
    let Some((mode, name)) = string_mode_name.split_once(' ') else {
        return Err(CommandError::InvalidMode);
    };
    return Ok((mode.to_string(), name.to_string()));
}

fn get_hash(stream: &mut dyn Read) -> Result<String, CommandError> {
    let mut hash = vec![0; 20];
    stream
        .read_exact(&mut hash)
        .map_err(|_| CommandError::ObjectHashNotKnown)?;
    let hash_str = hash
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("");
    Ok(hash_str)
}

impl GitObjectTrait for Tree {
    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        None
    }
    fn get_path(&self) -> Option<String> {
        Some(self.path.clone())
    }
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn as_tree(&mut self) -> Option<Tree> {
        Some(self.to_owned())
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "tree".to_string()
    }

    fn content(&mut self) -> Result<Vec<u8>, CommandError> {
        let mut sorted_objects = self.sort_objects();
        let mut content = Vec::new();
        for (name_object, object) in sorted_objects.iter_mut() {
            let mode = &object.mode();
            let mode_id = mode.get_id_mode();
            // println!("{} {}", mode_id, name_object);
            write!(content, "{} {}\0", mode_id, name_object)
                .map_err(|err| CommandError::FileWriteError(format!("{err}")))?;
            let hash = object.get_hash()?;
            content.extend_from_slice(&hash);
        }

        Ok(content)
    }

    fn add_path(
        &mut self,
        logger: &mut Logger,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        self.hash = None;
        let current_path_str = vector_path[..current_depth + 1].join("/");
        if current_depth != vector_path.len() - 1 {
            _ = self.add_tree(logger, vector_path, current_depth, hash)?;
            //_ = objects_database::write(logger, &mut tree)?;
        } else {
            self.add_blob(logger, &current_path_str, hash)?;
        }
        Ok(())
    }

    fn mode(&self) -> Mode {
        Mode::Tree
    }

    fn to_string_priv(&mut self) -> String {
        "ASDF".to_string()
        // format!(
        //     "{} {} {:?}    {:?}\n",
        //     self.mode(),
        //     self.type_str(),
        //     self.hash(),
        //     self.filename()
        // )
    }

    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        if let Some(hash) = self.hash {
            return Ok(hash);
        }
        let mut buf: Vec<u8> = Vec::new();
        self.write_to(&mut buf)?;
        let hash = get_sha1(&buf);
        self.set_hash(hash);
        Ok(hash)
    }
}

// impl fmt::Display for Tree {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.to_string_priv())
//     }
// }

fn type_id_object(type_str: &str) -> Result<u8, CommandError> {
    if type_str == "blob" {
        return Ok(0);
    }
    if type_str == "tree" {
        return Ok(1);
    }
    if type_str == "commit" {
        return Ok(2);
    }
    if type_str == "tag" {
        return Ok(3);
    }
    Err(CommandError::ObjectTypeError)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn given_a_path_a_tree_is_created() {
        let path = "test".to_string();
        let tree = Tree::new(path.clone());
        assert_eq!(tree.path, path);
    }

    #[test]
    fn given_a_path_a_tree_is_created_with_empty_objects() {
        let path = "test".to_string();
        let tree = Tree::new(path.clone());
        assert_eq!(tree.objects.len(), 0);
    }

    #[test]
    fn given_a_path_a_tree_is_created_with_empty_objects_and_then_add_a_blob() {
        let file_name = "test.txt".to_string();
        let mut logger = Logger::new_dummy();
        let current_dir = env::current_dir().unwrap();
        println!("The current directory is {}", current_dir.display());

        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        tree.add_blob(&mut logger, &file_name, &hash).unwrap();
        assert_eq!(tree.objects.len(), 1);
    }

    #[test]
    fn hhh() {
        let files = [
            "dir0/dir1/dir2/bar.txt".to_string(),
            "dir0/dir1/foo.txt".to_string(),
            "dir0/baz.txt".to_string(),
            "fu.txt".to_string(),
        ];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }
        let mut dir0 = tree.objects.get_mut("dir0").unwrap().as_tree().unwrap();
        assert!(&dir0.objects.contains_key("baz.txt"));
        let mut dir1 = dir0.objects.get_mut("dir1").unwrap().as_tree().unwrap();
        assert!(&dir1.objects.contains_key("foo.txt"));
        let dir2 = dir1.objects.get_mut("dir2").unwrap().as_tree().unwrap();
        assert!(&dir2.objects.contains_key("bar.txt"));
        assert!(&tree.objects.contains_key("fu.txt"));
    }

    #[test]
    #[ignore]
    fn hash_blob_from_path_true() {
        let files = [
            "dir0/dir1/dir2/bar.txt".to_string(),
            "dir0/dir1/foo.txt".to_string(),
            "dir0/baz.txt".to_string(),
            "fu.txt".to_string(),
        ];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }

        /* let result = tree.has_blob_from_path("dir0/dir1/dir2/bar.txt");
        assert!(result) */
    }

    #[test]
    #[ignore]
    fn hash_blob_from_path_false() {
        let files = [
            "dir0/dir1/dir2/bar.txt".to_string(),
            "dir0/dir1/foo.txt".to_string(),
            "dir0/baz.txt".to_string(),
            "fu.txt".to_string(),
        ];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }
        /* let result = tree.has_blob_from_path("dir0/dir1/dir2/barrrr.txt");

        assert!(!result) */
    }

    #[test]
    #[ignore]
    fn get_deleted_files() {
        let files = [
            "dir0/dir1/dir2/bar.txt".to_string(),
            "dir0/dir1/foo.txt".to_string(),
            "dir0/baz.txt".to_string(),
            "fu.txt".to_string(),
        ];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }

        let files: Vec<&str> = [
            "no1",
            "dir0/dir1/dir2/bar.txt",
            "dir0/dir1/foo.txt",
            "dir0/dir1/dir2/no-existe",
            "dir0/baz.txt",
            "fu.txt",
            "dir0/dir1/dir2/bar.txtt",
        ]
        .to_vec();

        let expected: Vec<String> = [
            "no1".to_string(),
            "dir0/dir1/dir2/no-existe".to_string(),
            "dir0/dir1/dir2/bar.txtt".to_string(),
        ]
        .to_vec();

        /* let result = tree.get_deleted_blobs_from_path2(files);

        assert_eq!(result, expected); */
    }
}

#[cfg(test)]
mod test_write_y_display {
    use std::io::{Cursor, Seek, SeekFrom};

    use crate::commands::objects::git_object::{self, read_git_object_from};

    use super::*;
    #[test]
    fn test_write_and_content() {
        let files = ["dir0/baz.txt".to_string(), "fu.txt".to_string()];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }

        let mut content = Vec::new();
        let mut writer_stream = Cursor::new(&mut content);
        tree.write_to(&mut writer_stream).unwrap();
        assert!(!content.is_empty());

        let mut reader_stream = Cursor::new(&mut content);
        let mut output = Vec::new();
        let mut output_stream = Cursor::new(&mut output);
        git_object::display_from_stream(
            &mut reader_stream,
            &mut Logger::new_dummy(),
            &mut output_stream,
        )
        .unwrap();

        let Ok(output_str) = String::from_utf8(output) else {
            panic!("Error");
        };

        assert_eq!(output_str, "040000 tree 378018f53fc0a1a74b2c85ec8481cdeae21df194    dir0\n100644 blob 30d74d258442c7c65512eafab474568dd706c430    fu.txt\n".to_string());
    }

    #[test]
    #[ignore]
    fn test_read_and_write() {
        let files = ["dir0/baz.txt".to_string(), "fu.txt".to_string()];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        let mut logger = Logger::new_dummy();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(&mut logger, vector_path, current_depth, &hash);
        }

        let mut content = Vec::new();
        let mut writer_stream = Cursor::new(&mut content);
        tree.write_to(&mut writer_stream).unwrap();

        writer_stream.seek(SeekFrom::Start(0)).unwrap();

        let mut tree_res = Tree::read_from(&mut writer_stream, 0, "", &hash, &mut logger).unwrap();
        //assert_eq!(tree_res, Box::new(tree));
        /* if let Some(tree_new) = tree_res.as_tree() {
            let x = tree_new.clone();
        } */
    }
}
