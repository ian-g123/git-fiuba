use std::{
    fs::File,
    io::{Bytes, Read},
};

use crate::{commands::command_errors::CommandError, logger::Logger};

use super::{
    aux::*,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
    tree::Tree,
};

#[derive(Clone, Debug)]
pub struct Blob {
    mode: Mode,
    path: String,
    hash: String,
    name: String,
    content: Vec<u8>,
}

impl Blob {
    /// Crea un Blob a partir de su ruta. Si la ruta no existe, devuelve Error.
    pub fn new(path: String) -> Result<Self, CommandError> {
        let object_type = "blob";
        let mode = Mode::get_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string(), false)?;

        Ok(Blob {
            mode: mode,
            path: path.clone(),
            hash: sha1,
            name: get_name(&path)?,
            content: Vec::new(),
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash(hash: String, path: String) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.clone())?;

        Ok(Blob {
            mode: mode,
            path: path.clone(),
            hash: hash,
            name: get_name(&path)?,
            content: Vec::new(),
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash_and_content(
        hash: &str,
        path: &str,
        content: Vec<u8>,
    ) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.to_string())?;

        Ok(Blob {
            mode: mode,
            path: path.to_string(),
            hash: hash.to_string(),
            name: get_name(&path.to_string())?,
            content,
        })
    }

    /// Devuelve el hash del Blob.
    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn read_from(
        stream: &mut dyn Read,
        path: &str,
        hash: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        // read until encountering \0
        logger.log("read_from");

        let mut bytes = stream.bytes();
        let type_str = {
            let end = ' ' as u8;
            let mut result = String::new();
            let Some(Ok(mut byte)) = bytes.next() else {
                return Err(CommandError::FileReadError(
                    "Error leyendo bytes".to_string(),
                ));
            };
            while byte != end {
                logger.log(&format!("byte: {}", byte));
                result.push(byte as char);
                let Some(Ok(byte_h)) = bytes.next() else {
                    return Err(CommandError::FileReadError(
                        "Error leyendo bytes".to_string(),
                    ));
                };
                byte = byte_h;
            }
            Ok(result)
        }?;
        if type_str != "blob" {
            return Err(CommandError::ObjectTypeError);
        }
        logger.log("Ping");
        let len_str = {
            let mut result = String::new();
            let Some(Ok(mut byte)) = bytes.next() else {
                return Err(CommandError::FileReadError(
                    "Error leyendo bytes".to_string(),
                ));
            };
            while byte != 0 {
                result.push(byte as char);
                let Some(Ok(byte_h)) = bytes.next() else {
                    return Err(CommandError::FileReadError(
                        "Error leyendo bytes".to_string(),
                    ));
                };
                byte = byte_h;
            }
            Ok(result)
        }?;
        let len: usize = len_str
            .parse()
            .map_err(|_| CommandError::ObjectLengthParsingError)?;

        let mut content = vec![0; len as usize];

        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;

        let blob = Blob::new_from_hash_and_content(hash, path, content)?;
        Ok(Box::new(blob))
    }
}

impl GitObjectTrait for Blob {
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "blob".to_string()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        read_file_contents(&self.path)
    }

    // TODO: implementar otros modos para blobs
    fn mode(&self) -> Mode {
        Mode::RegularFile
    }

    fn to_string(&self) -> &str {
        self.content
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("")
            .as_str()
    }
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}
