use std::{fs::File, io::Read, os::unix::prelude::PermissionsExt};

use crate::commands::command_errors::CommandError;

use super::{
    aux::*,
    git_object::{GitObject, GitObjectTree},
    mode::Mode,
    tree::Tree,
};

#[derive(Clone, Debug)]
pub struct Blob {
    mode: Mode,
    path: String,
    hash: String,
    name: String,
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
        })
    }

    /// Devuelve el hash del Blob.
    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }
}

impl GitObjectTree for Blob {
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
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}
