use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

extern crate sha1;
use crate::commands::{
    command_errors::CommandError, hash_object_components::hash_object::HashObject,
};
use sha1::{Digest, Sha1};

/// Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
pub fn get_name(path_string: &String) -> Result<String, CommandError> {
    let path = Path::new(path_string);
    if !path.exists() {
        return Err(CommandError::FileNotFound(path_string.to_string()));
    }
    if let Some(file_name) = path.file_name() {
        if let Some(name_str) = file_name.to_str() {
            return Ok(name_str.to_string());
        }
    }
    Err(CommandError::FileNotFound(path_string.to_owned()))
}

/// Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
pub fn get_name_bis(path_string: &String) -> Result<String, CommandError> {
    path_string
        .split("/")
        .last()
        .map(|s| s.to_string())
        .ok_or_else(|| CommandError::FileNotFound(path_string.to_owned()))
}

/* /// Devuelve el hash del path pasado. Si no existe, devuelve Error.
pub fn get_sha1(path: String, object_type: String) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, false, false);
    let (hash, _) = hash_object.run_for_content(content)?;
    Ok(hash)
} */

pub fn get_sha1_str(data: &[u8]) -> String {
    let hash_result = get_sha1(data);

    // Formatea los bytes del hash en una cadena hexadecimal
    let hex_string = u8_vec_to_hex_string(&hash_result);

    hex_string
}

pub fn get_sha1(data: &[u8]) -> [u8; 20] {
    let mut sha1 = Sha1::new();
    sha1.update(data);
    let hash_result: [u8; 20] = sha1.finalize().into();
    hash_result
}

/// Lee el contenido de un archivo y lo devuelve. Si la operación falla, devuelve error.
pub fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

/// Convierte de hexadecimal a Vec<u8>.
pub fn hex_string_to_u8_vec(hex_string: &str) -> [u8; 20] {
    let mut result = [0; 20];
    let mut chars = hex_string.chars();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.next() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                i += 1;
            } else {
                panic!("Invalid hex string");
            }
        } else {
            break;
        }
    }

    result
}

pub fn u8_vec_to_hex_string(u8_vec: &[u8]) -> String {
    let hex_string = u8_vec
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("");

    hex_string
}
pub trait SuperStrings {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError>;
    //fn read_from(&self, stream: &mut dyn Read) -> Result<String, CommandError>;

    fn cast_hex_to_u8_vec(&self) -> Result<[u8; 20], CommandError>;
}

impl SuperStrings for String {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let len_be = (self.len() as u32).to_be_bytes();
        stream
            .write_all(&len_be)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        stream.write_all(self.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(format!("Error escribiendo en el stream: {}", error))
        })?;
        Ok(())
    }

    /// Convierte de hexadecimal a Vec<u8>.
    fn cast_hex_to_u8_vec(&self) -> Result<[u8; 20], CommandError> {
        let mut result = [0; 20];
        let mut chars = self.chars();

        let mut i = 0;
        while let Some(c1) = chars.next() {
            if let Some(c2) = chars.next() {
                if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                    result[i] = (n1 * 16 + n2) as u8;
                    i += 1;
                } else {
                    panic!("Invalid hex string");
                }
            } else {
                break;
            }
        }

        Ok(result)
    }
}

pub fn read_string_from(stream: &mut dyn Read) -> Result<String, CommandError> {
    let mut len_be = [0; 4];
    stream
        .read_exact(&mut len_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let len = u32::from_be_bytes(len_be) as usize;

    let mut content = vec![0; len];
    stream
        .read_exact(&mut content)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;

    let result = String::from_utf8(content).map_err(|error| {
        CommandError::FileReadError(format!("Error leyendo el stream: {}", error))
    })?;
    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::{commands::command::Command, logger::Logger};
    use std::{env::current_dir, io::Cursor};

    use super::*;

    /// Prueba que la función get_name() devuelva el nombre correspondiente.
    #[test]
    fn get_name_test() {
        let Ok(path) = current_dir() else {
            assert!(false);
            return;
        };
        let Some(path) = path.to_str() else {
            assert!(false);
            return;
        };

        let res_expected = "git";
        match get_name(&path.to_string()) {
            Ok(result) => assert_eq!(res_expected, result),
            Err(error) => assert!(false, "{}", error),
        }
    }

    /// Prueba que la función get_name() falle si el path no existe.
    #[test]
    fn get_name_fails() {
        let path = String::from("no_existe");

        let res_expected = CommandError::FileNotFound(path.clone());
        match get_name(&path.to_string()) {
            Err(result) => assert_eq!(res_expected, result),
            Ok(result) => assert!(false, "Se obtuvo: {}", result),
        }
    }

    /// Prueba que la función read_file_contents() falle si el path no existe.
    #[test]
    fn read_fails() {
        let path = String::from("no_existe");

        let res_expected = CommandError::FileNotFound(path.clone());
        match read_file_contents(&path.to_string()) {
            Err(result) => assert_eq!(res_expected, result),
            Ok(_) => assert!(false),
        }
    }

    // Pureba que get_hash() obtenga correctamente el hash.
    // #[test]
    // fn get_hash_test() {
    //     let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
    //     let mut output_string = Vec::new();
    //     let mut stdout_mock = Cursor::new(&mut output_string);
    //     let path = "./src/main.rs".to_string();
    //     let input = "";
    //     let mut stdin_mock = Cursor::new(input.as_bytes());
    //     let args: &[String] = &["-t".to_string(), "blob".to_string(), path.clone()];
    //     assert!(HashObject::run_from(
    //         "hash-object",
    //         args,
    //         &mut stdin_mock,
    //         &mut stdout_mock,
    //         &mut logger
    //     )
    //     .is_ok());
    //     assert!(matches!(
    //         get_sha1(path, "blob".to_string()),
    //         Ok(_output_string)
    //     ))
    // }
}
