use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write},
    path::PathBuf,
};

use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{
    aux::{self, *},
    blob::Blob,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
};

//#[derive(Debug, Clone)]
#[derive(Clone)]
pub struct Tree {
    mode: Mode,
    path: String,
    hash: Option<String>,
    // name: String,
    objects: HashMap<String, GitObject>,
}

impl Tree {
    /// Crea un Tree a partir de su ruta y los objetos a los que referencia. Si la ruta no existe,
    /// devuelve Error.
    pub fn new(path: String, objects: HashMap<String, GitObject>) -> Result<Self, CommandError> {
        let object_type = "tree";
        let mode = Mode::get_mode(path.clone())?;
        // let sha1 = get_sha1(path.clone(), object_type.to_string(), false)?;
        let objects: HashMap<String, GitObject> = HashMap::new();

        Ok(Tree {
            mode: mode,
            path: path.clone(),
            hash: None,
            // name: get_name(&path)?,
            objects: objects,
        })
    }

    pub fn new_from_path(path: &str) -> Result<Self, CommandError> {
        let objects: HashMap<String, GitObject> = HashMap::new();

        let mode = Mode::Tree;
        Ok(Tree {
            mode,
            path: path.to_string(),
            hash: None,
            // name: get_name(path).unwrap(),
            objects: objects,
        })
    }

    /// Devuelve el hash del Tree.
    pub fn get_hash_interno(&self) -> String {
        todo!()
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let parent_path = get_parent(path_name)?;

        if self.path == parent_path {
            let blob = Blob::new_from_hash(hash.to_owned(), path_name.to_owned())?;
            _ = self.objects.insert(path_name.to_string(), Box::new(blob));
            return Ok(());
        }
        if parent_path.starts_with(&self.path) {
            return self.add_blob_to_subtree(path_name, &hash);
        }
        Err(CommandError::NotYourFather)
    }

    /// Agrega un Objeto Blob o Tree al Tree.
    fn insert(&mut self, path: &str, object: GitObject) {
        _ = self.objects.insert(path.to_string(), object);
    }

    /// Busca el Tree donde debe guardarse el Blob.
    fn add_blob_to_subtree(
        &mut self,
        path_name: &String,
        hash: &String,
    ) -> Result<(), CommandError> {
        for (_, object) in self.objects.iter_mut() {
            let Some(mut tree) = object.as_mut_tree() else {
                continue;
            };
            match tree.add_blob(path_name, hash) {
                Ok(()) => return Ok(()),
                Err(CommandError::NotYourFather) => continue,
                Err(error) => return Err(error),
            };
        }
        let child_tree = self.add_new_blob_in_new_tree(path_name, &hash)?;
        self.insert(path_name, Box::new(child_tree));
        Ok(())
    }

    fn add_new_blob_in_new_tree(&self, path_name: &str, hash: &str) -> Result<Tree, CommandError> {
        let parent_path = get_parent(path_name)?;
        let blob = Blob::new_from_hash(hash.to_string(), path_name.to_string())?;
        let mut tree = Tree::new_from_path(&parent_path)?;
        tree.insert(path_name, Box::new(blob));
        self.add_tree_in_new_tree(&parent_path, tree)
    }

    fn add_tree_in_new_tree(&self, path_name: &str, tree: Tree) -> Result<Tree, CommandError> {
        let parent_path = get_parent(path_name)?;
        if parent_path == self.path {
            return Ok(tree);
        }
        let mut parent_tree = Tree::new_from_path(&parent_path)?;
        parent_tree.insert(path_name, Box::new(tree));
        self.add_tree_in_new_tree(&parent_path, parent_tree)
    }

    fn hash(&self) -> Result<String, CommandError> {
        self.hash.clone().ok_or(CommandError::ObjectHashNotKnown)
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        hash: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut objects = HashMap::<String, GitObject>::new();

        while let Ok(mode) = Mode::read_from(stream) {
            let type_src = {
                let mut type_buf = [0; 1];
                stream
                    .read_exact(&mut type_buf)
                    .map_err(|error| CommandError::InvalidMode)?;
                match type_buf {
                    [0] => "blob",
                    [1] => "tree",
                    [2] => "commit",
                    [3] => "tag",
                    _ => return Err(CommandError::ObjectTypeError),
                }
            };
            let mut hash = vec![0; 20];
            stream
                .read_exact(&mut hash)
                .map_err(|error| CommandError::ObjectHashNotKnown)?;
            let hash_str = hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<Vec<_>>()
                .join("");

            let mut size_be = [0; 4];
            stream
                .read_exact(&mut size_be)
                .map_err(|error| CommandError::FailToCalculateObjectSize)?;
            let size = u32::from_be_bytes(size_be) as usize;
            let mut name = vec![0; size];
            stream
                .read_exact(&mut name)
                .map_err(|error| CommandError::FailToOpenSatginArea(error.to_string()))?;

            let object = objects_database::read_object(&hash_str, logger)?;
            objects.insert(hash_str, object);
        }
        Ok(Box::new(Self {
            mode: Mode::get_mode(path.to_string())?,
            path: path.to_string(),
            hash: None,
            objects,
        }))
    }

    pub fn filename(&self) -> Result<String, CommandError> {
        aux::get_name(&self.path)
    }
    pub(crate) fn display_from_hash(
        stream: &mut dyn Read,
        _: usize,
        _: String,
        _: &str,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut objects = Vec::<(Mode, String, String, String)>::new();

        while let Ok(mode) = Mode::read_from(stream) {
            let type_src = {
                let mut type_buf = [0; 1];
                stream
                    .read_exact(&mut type_buf)
                    .map_err(|error| CommandError::InvalidMode)?;
                match type_buf {
                    [0] => "blob",
                    [1] => "tree",
                    [2] => "commit",
                    [3] => "tag",
                    _ => return Err(CommandError::ObjectTypeError),
                }
            };
            let mut hash = vec![0; 20];
            stream
                .read_exact(&mut hash)
                .map_err(|error| CommandError::ObjectHashNotKnown)?;
            let hash_str = hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<Vec<_>>()
                .join("");

            let mut size_be = [0; 4];
            stream
                .read_exact(&mut size_be)
                .map_err(|_| CommandError::FailToCalculateObjectSize)?;
            let size = u32::from_be_bytes(size_be) as usize;
            let mut name = vec![0; size];
            stream
                .read_exact(&mut name)
                .map_err(|error| CommandError::FailToOpenSatginArea(error.to_string()))?;

            let object = objects_database::read_object(&hash_str, logger)?;
            let name_str = String::from_utf8(name).map_err(|_| CommandError::FileNameError)?;
            objects.push((mode, type_src.to_string(), hash_str, name_str));
        }
        for (mode, type_str, hash, name) in objects {
            writeln!(output, "{} {} {}    {}", mode, type_str, hash, name)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }
}

fn get_parent(path_name: &str) -> Result<String, CommandError> {
    let path: PathBuf = PathBuf::from(path_name);
    match path.parent() {
        Some(parent_path) => match parent_path.to_str() {
            Some(parent_path) => Ok(parent_path.to_owned()),
            None => return Err(CommandError::FileNotFound(path_name.to_owned())),
        },
        None => return Err(CommandError::FileNotFound(path_name.to_owned())),
    }
}

impl GitObjectTrait for Tree {
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "tree".to_string()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        let mut content = Vec::new();
        for (path, object) in self.objects.iter() {
            let hash_str = objects_database::write(object.to_owned())?;
            let filename = get_name(path)?;
            let type_byte = type_byte(&self.type_str())?;
            object.mode().write_to(&mut content)?;
            content.extend_from_slice(&[type_byte]);
            let hash_hex = hex_string_to_u8_vec(&hash_str);
            content.extend_from_slice(&hash_hex);
            let size_be = (filename.len() as u32).to_be_bytes();
            content.extend_from_slice(&size_be);
            content.extend_from_slice(filename.as_bytes());
        }
        Ok(content)
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

    // Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
    // pub fn get_name(&s)-> Result<String, CommandError>{

    //     let path = Path::new(path);
    //     if !path.exists(){
    //         return Err(CommandError::FileNotFound(path.to_string()));
    //     }
    //     if let Some(file_name) = path.file_name() {
    //         if let Some(name_str) = file_name.to_str() {
    //             return Ok(name_str.to_string());
    //         }
    //     }
    //     Err(CommandError::FileNotFound(path.to_owned()))
    // }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_priv())
    }
}

fn type_byte(type_str: &str) -> Result<u8, CommandError> {
    match type_str {
        "blob" => Ok(0),
        "tree" => Ok(1),
        "commit" => Ok(2),
        "tag" => Ok(3),
        _ => return Err(CommandError::ObjectTypeError),
    }
}
