use std::io::Read;
use std::path::Path;
use std::os::unix::prelude::PermissionsExt;
use std::fs::{self as _fs, File};

use crate::commands::command_errors::CommandError;
use crate::commands::hash_object_components::hash_object::HashObject;

use super::mode::Mode;

pub fn get_name(path_string: &String)-> Result<String, CommandError>{

    let path = Path::new(path_string);
    if !path.exists(){
        return Err(CommandError::FileNotFound(path_string.to_string()));
    }
    if let Some(file_name) = path.file_name() {
        if let Some(name_str) = file_name.to_str() {
            return Ok(name_str.to_string());
        }
    } 
    Err(CommandError::FileNotFound(path_string.to_owned()))
}

pub fn get_sha1(path: String, object_type: String, write: bool) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, write, false);
    let (hash, _) = hash_object.run_for_content(content)?;
    Ok(hash)
}


pub fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

pub fn set_mode(path: String)->Result<Mode, CommandError>{
    let mode: Mode;
    let Ok(metadata) = _fs::metadata(path.clone()) else{
        return Err(CommandError::FileNotFound(path));
    };
    let permissions_mode= metadata.permissions().mode();
    if metadata.is_dir(){
        mode = Mode::Tree;
    } else if metadata.is_symlink(){
        mode = Mode::SymbolicLink;
    } else if (permissions_mode & 0o111) != 0{
        mode = Mode::ExecutableFile;
    }else{
        mode = Mode::RegularFile;
    }
    Ok(mode)
}

#[cfg(test)]
mod test{
    use std::{env::current_dir, path::PathBuf, io};
    use super::*;

}