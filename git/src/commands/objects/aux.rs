use std::io::Read;
use std::path::Path;
use std::fs::File;

use crate::commands::command_errors::CommandError;
use crate::commands::hash_object_components::hash_object::HashObject;

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

/// Devuelve el hash del path pasado. Si no existe, devuelve Error.
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
    use std::{env::current_dir, io::Cursor};
    use crate::{commands::command::Command, logger::Logger};

    use super::*;

    /// Prueba que la función get_name() devuelva el nombre correspondiente.
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
        match get_name(&path.to_string()){
            Ok(result) => assert_eq!(res_expected, result),
            Err(error) => assert!(false, "{}", error)
        }
    }

    /// Prueba que la función get_name() falle si el path no existe.
    #[test]
    fn get_name_fails(){
        let path = String::from("no_existe");

        let res_expected =CommandError::FileNotFound(path.clone());
        match get_name(&path.to_string()){
            Err(result) => assert_eq!(res_expected, result),
            Ok(result) => assert!(false, "Se obtuvo: {}", result)
        }
    }

    /// Prueba que la función read_file_contents() falle si el path no existe.
    #[test]
    fn read_fails(){
        let path = String::from("no_existe");

        let res_expected =CommandError::FileNotFound(path.clone());
        match read_file_contents(&path.to_string()){
            Err(result) => assert_eq!(res_expected, result),
            Ok(_) => assert!(false)
        }    
    }

    /// Pureba que get_hash() obtenga correctamente el hash.
    #[test]
    fn get_hash_test(){
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);
        let path = "./src/main.rs".to_string();
        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            path.clone(),
        ];
        assert!(HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());        
        assert!(matches!(get_sha1(path, "blob".to_string(), false), Ok(_output_string)))
    }
}


