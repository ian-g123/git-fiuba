use std::{fs::File, io::Read};

extern crate sha1;
use crate::commands::command_errors::CommandError;
use sha1::{Digest, Sha1};

/// Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
pub fn get_name(path_string: &str) -> Result<String, CommandError> {
    path_string
        .split("/")
        .last()
        .map(|s| s.to_string())
        .ok_or_else(|| CommandError::FileNotFound(path_string.to_owned()))
}

/// Dado el contenido pasado, devuelve el hash expresado en un vector de 20 bytes.
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

#[cfg(test)]
mod test {
    use std::env::current_dir;

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
