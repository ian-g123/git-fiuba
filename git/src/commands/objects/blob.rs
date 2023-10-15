use std::os::unix::prelude::PermissionsExt;

use crate::commands::command_errors::CommandError;

use super::{mode::Mode, aux::*};

#[derive(Clone, Debug)]
pub struct Blob{
    mode: Mode,
    path: String,
    hash: String,
    name: String,
}

impl Blob{

    /// Crea un Blob a partir de su ruta. Si la ruta no existe, devuelve Error.
    pub fn new(path: String)-> Result<Self, CommandError>{
        let object_type = "blob";
        let mode = Mode::get_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string(), false)?;
        Ok(Blob{
            mode: mode, path: path.clone(), hash:sha1,
            name: get_name(&path)?
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash(hash:String, path:String)-> Result<Self, CommandError>{
        let mode = Mode::get_mode(path.clone())?;
        Ok(Blob{
            mode: mode, path: path.clone(), hash:hash,
            name: get_name(&path)?
        })
    }

    /// Devuelve el hash del Blob.
    pub fn get_hash(&self)->String{
        self.hash.clone()
    }
}