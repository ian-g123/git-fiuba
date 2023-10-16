use super::{
    aux::*,
    blob::Blob,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
};
use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};
use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write},
    path::Path,
};

#[derive(Clone)]
pub struct Tree {
    path: String,
    objects: HashMap<String, GitObject>,
}

impl Tree {
    /// Crea un Tree a partir de su ruta y los objetos a los que referencia. Si la ruta no existe,
    /// devuelve Error.
    pub fn new(path: String) -> Result<Self, CommandError> {
        Ok(Tree {
            path: path.clone(),
            objects: HashMap::new(),
        })
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let blob = Blob::new_from_hash(hash.to_owned(), path_name.to_owned())?;
        _ = self.objects.insert(path_name.to_string(), Box::new(blob));
        Ok(())
    }

    pub fn add_tree(&mut self, path_str: &String) -> Result<(), CommandError> {
        let path_name = get_name(path_str)?;
        if self.objects.contains_key(&path_name) {
            return Ok(());
        };
        let tree = Tree::new(path_name.to_owned())?;
        _ = self.objects.insert(path_name.to_string(), Box::new(tree));
        Ok(())
    }

    pub fn add_path(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let part_path = path_name.split("/").collect::<Vec<_>>();

        let mut current_path_str = "".to_string();
        for part in part_path {
            current_path_str = format!("{}/{}", current_path_str, part);
            let current_path = Path::new(&current_path_str);

            if current_path.is_dir() {
                self.add_tree(&current_path_str)?;
            } else {
                self.add_blob(&current_path_str, hash)?;
            }
        }
        Ok(())
    }

    fn get_data(&self) -> Result<Vec<u8>, CommandError> {
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

        while let Ok(mode) = Mode::read_from(stream) {
            logger.log(&format!("mode: {:?}", mode));
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

            logger.log(&format!("objects_database::read : {:?}", hash_str));
            let object = objects_database::read_object(&hash_str, logger)?;
            logger.log(&format!("Success! : {:?}", object));
            objects.insert(hash_str, object);
        }
        Ok(Box::new(Self {
            path: path.to_string(),
            objects,
        }))
    }

    pub(crate) fn display_from_hash(
        stream: &mut dyn Read,
        len: usize,
        path: String,
        hash: &str,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut objects = Vec::<(Mode, String, String, String)>::new();

        while let Ok(mode) = Mode::read_from(stream) {
            logger.log(&format!("mode: {:?}", mode));
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

            logger.log(&format!("objects_database::read : {:?}", hash_str));
            let object = objects_database::read_object(&hash_str, logger)?;
            logger.log(&format!("Success! : {:?}", object));
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

fn hex_string_to_u8_vec(hex_string: &str) -> [u8; 20] {
    let mut result = [0; 20];
    let mut chars = hex_string.chars().peekable();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.peek() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                chars.next();
                i += 1;
            } else {
                panic!("Invalid hex string");
            }
        } else {
            panic!("Invalisd hex string");
        }
    }

    result
}
