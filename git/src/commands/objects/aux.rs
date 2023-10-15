use std::io::Read;
use std::path::Path;
use std::fs::File;

use crate::commands::command_errors::CommandError;
use crate::commands::hash_object_components::hash_object::HashObject;

use super::mode::Mode;

/// Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
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

/// Lee el contenido de un archivo y lo devuelve. Si la operación falla, devuelve error.
pub fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

#[cfg(test)]
mod test{
    use std::{env::current_dir, path::PathBuf, io, f32::consts::E};
    use super::*;

    #[test]
    fn get_name_test(){
        let Ok(path) = current_dir() else{
            assert!(false);
            return;
        };
        let Some(path) = path.to_str() else{
            assert!(false);
            return;
        };

        let res_expected = "git";
        assert!(matches!(get_name(&path.to_string()), res_expected))
    }

    #[test]
    fn get_name_fails(){
        let path = String::from("no_existe");

        let res_expected =CommandError::FileNotFound(path.clone());
        assert!(matches!(get_name(&path), res_expected))
    }

    #[test]
    fn read_fails(){
        let path = String::from("no_existe");

        let res_expected =CommandError::FileNotFound(path.clone());
        assert!(matches!(read_file_contents(&path), res_expected))
    }
}