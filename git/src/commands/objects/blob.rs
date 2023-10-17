use std::{
    fmt,
    fs::File,
    io::{Cursor, Read, Write},
};

use sha1::Sha1;

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
    path: Option<String>,
    hash: Option<[u8; 20]>,
    name: Option<String>,
    content: Option<Vec<u8>>,
}

impl Blob {
    /// Crea un Blob a partir de su ruta. Si la ruta no existe, devuelve Error.
    pub fn new_from_path(path: String) -> Result<Self, CommandError> {
        let object_type = "blob";
        let mode = Mode::get_mode(path.clone())?;
        // let sha1 = get_sha1(path.clone(), object_type.to_string(), false)?;
        // let data: Vec<u8> = Vec::new();

        Ok(Blob {
            mode: mode,
            path: Some(path.clone()),
            hash: None,
            name: Some(get_name(&path)?),
            content: None,
        })
    }

    /* /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash(hash: String, path: String) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.clone())?;

        Ok(Blob {
            mode: mode,
            path: path.clone(),
            hash: hash,
            name: get_name(&path)?,
            content: None,
        })
    } */

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
            name: Some(get_name_bis(&path)?),
            content: None,
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash_and_content(
        hash: &str,
        path: &str,
        content: Vec<u8>,
    ) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.to_string())?;
        let hash = hash.to_string().cast_hex_to_u8_vec()?;
        Ok(Blob {
            mode: mode,
            path: Some(path.to_string()),
            hash: Some(hash),
            name: Some(get_name(&path.to_string())?),
            content: Some(content),
        })
    }

    pub fn new_from_content(content: Vec<u8>) -> Result<Self, CommandError> {
        let mode = Mode::RegularFile;
        Ok(Blob {
            mode: mode,
            path: None,
            hash: None,
            name: None,
            content: Some(content),
        })
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        hash: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        // read until encountering \0

        let mut content = vec![0; len as usize];

        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        logger.log(&format!(
            "content: {}",
            String::from_utf8(content.clone()).unwrap()
        ));

        let blob = Blob::new_from_hash_and_content(hash, path, content)?;
        logger.log("blob created");
        logger.log(&format!("blob: {}", blob.to_string()));
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
        let output_str = String::from_utf8(content).map_err(|error| {
            logger.log("Error convierttiendo a utf8 blob");
            CommandError::FileReadError(error.to_string())
        })?;
        writeln!(output, "{}", output_str)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
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
        match &self.content {
            Some(content) => Ok(content.clone()),
            None => match &self.path {
                Some(path) => read_file_contents(path),
                None => Err(CommandError::FileReadError(
                    "Error leyendo el archivo".to_string(),
                )),
            },
        }
    }

    // TODO: implementar otros modos para blobs
    fn mode(&self) -> Mode {
        Mode::RegularFile
    }

    fn to_string_priv(&self) -> String {
        //map content to utf8
        let Some(content) = &self.content else {
            return "Error convirtiendo a utf8".to_string();
        };
        let Ok(string) = String::from_utf8(content.clone()) else {
            return "Error convirtiendo a utf8".to_string();
        };
        string
    }

    /// Devuelve el hash del Blob.
    fn get_hash(&self) -> Result<[u8; 20], CommandError> {
        match self.hash {
            Some(hash) => Ok(hash),
            None => {
                let mut buf: Vec<u8> = Vec::new();
                let mut stream = Cursor::new(&mut buf);
                self.write_to(&mut stream)?;
                Ok(get_sha1(&buf))
            }
        }
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_priv())
    }
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}
