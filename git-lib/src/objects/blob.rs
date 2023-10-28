use std::{
    fs::File,
    io::{Cursor, Read, Write},
};

use crate::command_errors::CommandError;
use crate::logger::Logger;

use super::{
    author::Author,
    aux::*,
    git_object::{write_to_stream_from_content, GitObject, GitObjectTrait},
    mode::Mode,
    super_string::SuperStrings,
    tree::Tree,
};

#[derive(Clone, Debug)]
pub struct Blob {
    mode: Mode,
    path: Option<String>,
    hash: Option<[u8; 20]>,
    name: Option<String>,
}

impl Blob {
    /// Crea un Blob a partir de su ruta. Si la ruta no existe, devuelve Error.
    pub fn new_from_path(path: String) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.clone())?;
        Ok(Blob {
            mode: mode,
            path: Some(path.clone()),
            hash: None,
            name: Some(get_name(&path)?),
        })
    }

    pub fn new_from_hash_and_name(
        hash: String,
        name: String,
        mode: Mode,
    ) -> Result<Self, CommandError> {
        let hash = hash.cast_hex_to_u8_vec()?;
        Ok(Blob {
            mode,
            path: None,
            hash: Some(hash),
            name: Some(name),
        })
    }

    pub fn new_from_hash_and_mode(
        hash: String,
        path: String,
        mode: Mode,
    ) -> Result<Self, CommandError> {
        let hash = hash.cast_hex_to_u8_vec()?;
        Ok(Blob {
            mode,
            path: Some(path.clone()),
            hash: Some(hash),
            name: Some(get_name(&path)?),
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash_and_path(hash: [u8; 20], path: &str) -> Result<Self, CommandError> {
        let mode = if path != "" {
            Mode::get_mode(path.to_string())?
        } else {
            Mode::RegularFile
        };
        Ok(Blob {
            mode: mode,
            path: Some(path.to_string()),
            hash: Some(hash),
            name: Some(get_name(&path.to_string())?),
        })
    }

    pub fn new_from_content_and_path(content: Vec<u8>, path: &str) -> Result<Self, CommandError> {
        let mut data: Vec<u8> = Vec::new();
        write_to_stream_from_content(&mut data, content, "blob".to_string())?;
        let hash = get_sha1(&data);
        Self::new_from_hash_and_path(hash, path)
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        _: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut content = vec![0; len as usize];

        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        logger.log(&format!(
            "content: {}",
            String::from_utf8(content.clone()).unwrap()
        ));

        let blob = Blob::new_from_content_and_path(content, path)?;
        logger.log("blob created");
        Ok(Box::new(blob))
    }

    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        len: usize,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut content = vec![0; len as usize];
        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let output_str = String::from_utf8(content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        writeln!(output, "{}", output_str)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

impl GitObjectTrait for Blob {
    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        None
    }
    fn get_path(&self) -> Option<String> {
        self.path.clone()
    }
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "blob".to_string()
    }

    fn content(&mut self) -> Result<Vec<u8>, CommandError> {
        match &self.path {
            Some(path) => {
                let content = read_file_contents(path)?;
                Ok(content)
            }
            None => Err(CommandError::FileReadError(
                "Archivo blob inexistente".to_string(),
            )),
        }
    }

    // TODO: implementar otros modos para blobs
    fn mode(&self) -> Mode {
        Mode::RegularFile
    }

    fn to_string_priv(&mut self) -> String {
        //map content to utf8
        let Ok(content) = self.content() else {
            return "Error convirtiendo a utf8".to_string();
        };
        let Ok(string) = String::from_utf8(content.clone()) else {
            return "Error convirtiendo a utf8".to_string();
        };
        string
    }

    /// Devuelve el hash del Blob.
    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        match self.hash {
            Some(hash) => Ok(hash),
            None => {
                let mut buf: Vec<u8> = Vec::new();
                let mut stream = Cursor::new(&mut buf);
                self.write_to(&mut stream)?;
                let sha1 = get_sha1(&buf);
                self.hash = Some(sha1.clone());
                Ok(sha1)
            }
        }
    }

    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(content)
}
