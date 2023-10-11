use std::fs::File;
use std::io::Read;
use std::io::{self, Write};
use std::str;

extern crate sha1;

use sha1::{Digest, Sha1};

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;

pub struct HashObject {
    object_type: String,
    write: bool,
    path: String,
    stdin: bool,
}

impl Command for HashObject {
    fn run_from(name: &str, args: &[String], output: &mut dyn Write) -> Result<(), ErrorFlags> {
        if name != "hash-object" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args, output)?;

        instance.run(output)?;
        Ok(())
    }
}

impl HashObject {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Self, ErrorFlags> {
        let Some(path) = args.last() else {
            return Err(ErrorFlags::InvalidArguments);
        };

        let object_type = obtain_object_type(args)?;

        // let stdin = if args.contains(&"--stdin".to_string()) {
        //     true
        // } else {
        //     false
        // };

        // let arguments = obtain_arguments(args)?;

        let hash_object = HashObject {
            object_type,
            path: path.to_string(),
            write: false,
            stdin: false,
        };

        Ok(hash_object)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let content = read_file_contents(&self.path)?;
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        write!(output, "{}", hex_string);
        Ok(())
    }

    fn get_header(&self, data: &Vec<u8>) -> String {
        let length = data.len();
        format!("{} {}\0", self.object_type, length)
    }

    fn get_sha1(&self, data: &[u8]) -> String {
        let mut sha1 = Sha1::new();
        sha1.update(&data);
        let hash_result = sha1.finalize();

        // Formatea los bytes del hash en una cadena hexadecimal
        let hex_string = hash_result
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("");

        hex_string
    }
}

fn obtain_object_type(args: &[String]) -> Result<String, ErrorFlags> {
    let object_type = if args.contains(&"-t".to_string()) {
        let index = match args.iter().position(|x| x == "-t") {
            Some(index) => index,
            None => return Err(ErrorFlags::InvalidArguments),
        };
        if index + 1 >= args.len() {
            return Err(ErrorFlags::ObjectTypeError);
        }
        let arg = args[index + 1].clone();
        if arg == "blob" || arg == "commit" || arg == "tree" || arg == "tag" {
            arg
        } else {
            return Err(ErrorFlags::ObjectTypeError);
        }
    } else {
        "blob".to_string()
    };
    Ok(object_type)
}

fn obtain_arguments(args: &[String]) -> Result<Vec<String>, ErrorFlags> {
    let mut arguments = Vec::new();
    for arg in args {
        if arg.starts_with("--") {
            arguments.push(arg.to_string());
        }
    }
    Ok(arguments)
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, ErrorFlags> {
    let mut file = File::open(path).map_err(|_| ErrorFlags::FileNotFound)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| ErrorFlags::FileReadError)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);

        let args: &[String] = &[];
        assert!(matches!(
            HashObject::run_from("", args, &mut cursor),
            Err(ErrorFlags::CommandName)
        ));
    }

    #[test]
    fn test_path_null() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);

        let args: &[String] = &[];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut cursor),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    #[test]
    fn test_path() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);
        let args: &[String] = &["./test/commands/hash_object/codigo1.txt".to_string()];
        assert!(HashObject::run_from("hash-object", args, &mut cursor).is_ok());

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(HashObject::run_from("hash-object", args, &mut cursor).is_ok());

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type_error() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);
        let args: &[String] = &[
            "-t".to_string(),
            "blob2".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut cursor),
            Err(ErrorFlags::ObjectTypeError)
        ));
    }

    #[test]
    fn test_object_type_tree_error() {
        let mut output_string = Vec::new();
        let mut cursor = io::Cursor::new(&mut output_string);
        let args: &[String] = &[
            "-t".to_string(),
            "tree".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut cursor),
            Err(ErrorFlags::ObjectTypeError)
        ));
    }


    // #[test]
    // fn test_stdin() {
    //     let mut output_string = Vec::new();
    //     let mut cursor = io::Cursor::new(&mut output_string);
    //     let args: &[String] = &["--stdin".to_string()];
    //     assert!(HashObject::run_from("hash-object", args, &mut cursor).is_ok());

    //     let Ok(output) = String::from_utf8(output_string) else {
    //         panic!("Error");
    //     };

    //     let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59";
    //     assert_eq!(output, hex_git);
    // }
}
