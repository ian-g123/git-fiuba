use crate::commands::command_errors::CommandError;

use super::{mode::Mode, aux::*};

#[derive(Clone, Debug)]
pub struct Blob{
    mode: Mode,
    path: String,
    hash: String,
    name: String
}

impl Blob{
    fn new(path: String)-> Result<Self, CommandError>{
        let object_type = "blob";
        let mode = set_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string())?;
        Ok(Blob{
            mode: mode, path: path.clone(),
            name: get_name(path), hash:sha1
        })
    }

    pub fn get_hash(&self)->String{
        self.hash.clone()
    }
}