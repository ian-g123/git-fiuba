pub mod flags;
pub mod type_values;

use self::flags::FlagsHashObject;
use crate::{command::Command, error_args::ErrorArgs};

pub struct HashObject {}

// git hash-object -t blob --stdin -w  --path <file>

impl Command for HashObject {
    fn run(name: &str, args: &[String]) -> Result<(), ErrorArgs> {
        if name != "hash-object" {
            print!("El nombre no es hash-object");
            return Err(ErrorArgs::CommandName);
        }

        let mut recorded_values = Vec::<FlagsHashObject>::new();
        let mut current_flag = "";
        let mut values_buffer = Vec::<String>::new();

        for arg in args {
            if Self::is_flag(&arg) {
                if !current_flag.is_empty() {
                    match FlagsHashObject::get_flag(current_flag, values_buffer) {
                        Ok(value) => recorded_values.push(value),
                        Err(error) => return Err(error),
                    }
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }

        for value in recorded_values {
            println!("{}", value);
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_hash_object() {
//         let args = vec![
//             "-t".to_string(),
//             "blob".to_string(),
//             "--stdin".to_string(),
//             "-w".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//         ];
//         let result = HashObject::run("hash-object", &args);
//         assert_eq!(result, Ok(()));
//     }

//     #[test]
//     fn test_hash_object_invalid_flag() {
//         let args = vec![
//             "-t".to_string(),
//             "blob".to_string(),
//             "--stdin".to_string(),
//             "-w".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--invalid".to_string(),
//         ];
//         let result = HashObject::run("hash-object", &args);
//         assert_eq!(ErrorArgs::InvalidFlag, result.err().unwrap());
//     }

//     #[test]
//     fn test_hash_object_invalid_flag_value() {
//         let args = vec![
//             "-t".to_string(),
//             "blob".to_string(),
//             "--stdin".to_string(),
//             "-w".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--path".to_string(),
//         ];
//         let result = HashObject::run("hash-object", &args);
//         assert_eq!(result, Err(ErrorArgs::InvalidFlag));
//     }

//     #[test]
//     fn test_hash_object_invalid_flag_value2() {
//         let args = vec![
//             "-t".to_string(),
//             "blob".to_string(),
//             "--stdin".to_string(),
//             "-w".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--path".to_string(),
//         ];
//         let result = HashObject::run("hash-object", &args);
//         assert_eq!(result, Err(ErrorArgs::InvalidFlag));
//     }

//     #[test]
//     fn test_hash_object_invalid_flag_value3() {
//         let args = vec![
//             "-t".to_string(),
//             "blob".to_string(),
//             "--stdin".to_string(),
//             "-w".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//             "--path".to_string(),
//             "file.txt".to_string(),
//         ];
//         let result = HashObject::run("hash-object", &args);
//         assert_eq!(result, Err(ErrorArgs::InvalidFlag));
//     }
// }

/*
let mut hasher = Sha1::new();

// Texto que deseas hashear
let text = "Hola, mundo";

// Escribe el texto en el hasher
hasher.write(text.as_bytes());

// Calcula el hash SHA-1 y conviértelo en una cadena hexadecimal
let result = hasher.digest().to_string();

// Imprime el resultado
println!("Texto: {}", text);
println!("Hash SHA-1: {}", result);
*/

// git hash-object -t blob --stdin -w  --path <file>
// [-t, blob, --stdin, -w, --path, <file>]

// -t: [blob]
// --stdin: []
// -w: []
// --path: [<file>]
