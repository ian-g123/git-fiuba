use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write},
};

use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{
    aux::*,
    blob::Blob,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
};

#[derive(Clone)]
pub struct Tree {
    path: String,
    objects: HashMap<String, GitObject>,
}

impl Tree {
    pub fn add_path_tree(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        Ok(self.add_path(vector_path, current_depth, hash)?)
    }

    /// Crea un Tree a partir de su ruta y los objetos a los que referencia. Si la ruta no existe,
    /// devuelve Error.
    pub fn new(path: String) -> Tree {
        Tree {
            path: path.clone(),
            objects: HashMap::new(),
        }
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        // let blob = Blob::new_from_hash(hash.clone(), path_name.clone())?;
        let blob =
            Blob::new_from_hash_and_mode(hash.clone(), path_name.clone(), Mode::RegularFile)?;
        let blob_name = get_name_bis(&path_name)?;
        _ = self.objects.insert(blob_name.to_string(), Box::new(blob));
        Ok(())
    }

    pub fn add_tree(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        let current_path = vector_path[..current_depth + 1].join("/");
        let tree_name = get_name_bis(&current_path)?;

        if !self.objects.contains_key(&tree_name) {
            let tree = Tree::new(current_path.to_owned());
            self.objects.insert(tree_name.clone(), Box::new(tree));
        }

        let Some(tree) = self.objects.get_mut(&tree_name) else {
            return Err(CommandError::ObjectNotTree);
        };

        tree.add_path(vector_path, current_depth + 1, hash)?;
        Ok(())
    }

    fn get_data(&mut self) -> Result<Vec<u8>, CommandError> {
        let header = format!("1 {}\0", self.size()?);
        let content = self.content()?;
        Ok([header.as_bytes(), content.as_slice()].concat())
    }

    fn get_mode(&self) -> Result<Mode, CommandError> {
        Ok(Mode::get_mode(self.path.clone())?)
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        hash: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut objects = HashMap::<String, GitObject>::new();

        loop {
            let mut buf = Vec::new();
            if stream.read_to_end(&mut buf).is_err() {
                break;
            }
            let (_mode, _name) = get_mode_name(buf).map_err(|error| {
                CommandError::FileReadError("No se pudo leer objeto".to_string())
            })?;
            let hash_str = get_hash(stream)?;
            let object = objects_database::read_object(&hash_str, logger)?;
            objects.insert(hash_str, object);
        }

        Ok(Box::new(Self {
            path: path.to_string(),
            objects,
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
            let Ok(string_part_bytes) = read_to(0, stream) else {
                break;
            };

            let string_part_bytes = String::from_utf8(string_part_bytes).map_err(|error| {
                CommandError::FileReadError("No se pudo leer objeto".to_string())
            })?;

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
        }
        Ok(())
    }
}

fn get_mode(mode: &str) -> Result<Mode, CommandError> {
    let id = mode
        .parse::<u32>()
        .map_err(|error| CommandError::FileReadError("No se pudo leer objeto".to_string()))?;
    let mode = Mode::get_mode_from_id(id)
        .map_err(|error| CommandError::FileReadError("No se pudo leer objeto".to_string()))?;
    Ok(mode)
}

fn get_mode_name(buf: Vec<u8>) -> Result<(String, String), CommandError> {
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
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn as_tree(&self) -> Option<&Tree> {
        Some(self)
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "tree".to_string()
    }

    fn content(&mut self) -> Result<Vec<u8>, CommandError> {
        let mut content = Vec::new();
        for (name_object, object) in self.objects.iter_mut() {
            let mode = &object.mode();
            let mode_id = mode.get_id_mode();
            println!("{} {}", mode_id, name_object);
            write!(content, "{} {}\0", mode_id, name_object)
                .map_err(|err| CommandError::FileWriteError(format!("{err}")))?;
            let hash = object.get_hash()?;
            content.extend_from_slice(&hash);
        }
        Ok(content)
    }

    fn add_path(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        let current_path_str = vector_path[..current_depth + 1].join("/");
        if current_depth != vector_path.len() - 1 {
            self.add_tree(vector_path, current_depth, hash)?;
        } else {
            self.add_blob(&current_path_str, hash)?;
        }
        Ok(())
    }

    fn mode(&self) -> Mode {
        Mode::Tree
    }

    fn to_string_priv(&self) -> String {
        "ASDF".to_string()
        // format!(
        //     "{} {} {:?}    {:?}\n",
        //     self.mode(),
        //     self.type_str(),
        //     self.hash(),
        //     self.filename()
        // )
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_priv())
    }
}

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
        let current_dir = env::current_dir().unwrap();
        println!("The current directory is {}", current_dir.display());

        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        tree.add_blob(&file_name, &hash).unwrap();
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
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(vector_path, current_depth, &hash);
        }
        let dir0 = tree.objects.get("dir0").unwrap().as_tree().unwrap();
        let dir1 = dir0.objects.get("dir1").unwrap().as_tree().unwrap();
        let dir2 = dir1.objects.get("dir2").unwrap().as_tree().unwrap();
        assert!(&dir2.objects.contains_key("bar.txt"));
        assert!(&dir1.objects.contains_key("foo.txt"));
        assert!(&tree.objects.contains_key("fu.txt"));
        assert!(&dir0.objects.contains_key("baz.txt"));
    }
}

#[cfg(test)]
mod test_write_y_display {
    use std::io::Cursor;

    use crate::commands::objects::git_object;

    use super::*;
    #[test]
    #[ignore]
    fn test_write_and_content() {
        let files = ["dir0/baz.txt".to_string(), "fu.txt".to_string()];
        let mut tree = Tree::new("".to_string());
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(vector_path, current_depth, &hash);
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
}
