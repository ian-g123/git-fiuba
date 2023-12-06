use std::{fs::File, io::Read, path::Path};

extern crate sha1;
use crate::command_errors::CommandError;
use sha1::{Digest, Sha1};

use super::super_string::u8_vec_to_hex_string;

/// Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
pub fn get_name(path_string: &str) -> Result<String, CommandError> {
    let var = path_string
        .split('/')
        .last()
        .map(|s| s.to_string())
        .ok_or_else(|| CommandError::FileNotFound(path_string.to_owned()))?;

    Ok(var)
}

/// Dado la data pasado, devuelve el hash expresado en un vector de 20 bytes.
pub fn get_sha1_str(data: &[u8]) -> String {
    let hash_result = get_sha1(data);

    // Formatea los bytes del hash en una cadena hexadecimal

    u8_vec_to_hex_string(&hash_result)
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
                panic!("Panic Invalid hex string: '{}'", hex_string);
            }
        } else {
            break;
        }
    }

    result
}

/// Se obtiene una cadena de texto desde el stream hasta encontrar el caracter char_stop.
pub fn read_string_until(stream: &mut dyn Read, char_stop: char) -> Result<String, CommandError> {
    let string = {
        let mut bytes = stream.bytes();
        let end = char_stop as u8;
        let mut result = String::new();
        loop {
            if let Some(Ok(byte)) = bytes.next() {
                if byte == end {
                    break;
                }
                result.push(byte as char);
            } else {
                return Err(CommandError::FileReadError(format!(
                    "Error leyendo string hasta el caracter {}",
                    char_stop
                )));
            }
        }
        Ok(result)
    }?;
    Ok(string)
}

#[cfg(test)]
mod test {

    use super::*;

    /// Prueba que la función get_name() devuelva el nombre correspondiente.
    #[test]
    fn get_name_test() {
        let path = "dir/git";

        let res_expected = "git";
        match get_name(path) {
            Ok(result) => assert_eq!(res_expected, result),
            Err(error) => assert!(false, "{}", error),
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
    //     let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs.log").unwrap();
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

pub fn join_paths_m(base_path_str: &str, relative_path_str: &str) -> Result<String, CommandError> {
    let base_path: &Path = Path::new(base_path_str);
    let complete_path = &base_path.join(relative_path_str);
    let complete_path_str = complete_path.to_str().ok_or(CommandError::JoiningPaths)?;
    Ok(complete_path_str.to_string())
}

// pub fn join_paths_vec(paths: Vec<String>) -> Result<String, CommandError> {
//     let mut result = String::new();
//     for path in paths {
//         result = join_paths_m(&result, &path)?;
//     }
//     Ok(result)
// }

// create a macro join_paths! that receives a variable number of arguments and returns a String
// with the paths joined
#[macro_export]
macro_rules! join_paths {
    ($($x:expr),*) => {
        {
            let mut result = Some(String::new());

            $(
                match result {
                    Some(result_s) => {
                        let base_path: &std::path::Path = std::path::Path::new(&result_s);
                        #[allow(clippy::unnecessary_to_owned)]
                        let complete_path = &base_path.join(&$x);
                        match complete_path.to_str() {
                            Some(result_res) => result = Some(result_res.to_string()),
                            None => result = None,
                        };
                    }
                    None => {}
                }
            )*

            result
        }
    };
}
