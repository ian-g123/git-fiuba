use std::io::Read;
use std::os::unix::prelude::PermissionsExt;
use std::fs::{self as _fs, File};

use crate::commands::command_errors::CommandError;
use crate::commands::hash_object_components::hash_object::HashObject;

use super::mode::Mode;

pub fn get_name(path: String)-> String{
    let parts: Vec<&str> = path.split('/').collect();
    parts[parts.len()-1].to_string()
}

pub fn get_sha1(path: String, object_type: String) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, false, false);
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