use std::fs::File;
use std::io::{self, Write};
use std::io::{Read, Stdin};
use std::str;

extern crate sha1;

use sha1::{Digest, Sha1};

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;

/// Commando hash-object
pub struct HashObject {
    object_type: String,
    write: bool,
    file: Option<String>,
    stdin: bool,
}

impl Command for HashObject {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        if name != "hash-object" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args, output)?;

        instance.run(stdin, output)?;
        Ok(())
    }
}

impl HashObject {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Self, ErrorFlags> {
        let mut hash_object = Self::new_default();

        hash_object.config(args, output)?;

        Ok(hash_object)
    }

    fn new_default() -> Self {
        let mut hash_object = Self {
            object_type: "blob".to_string(),
            file: None,
            write: false,
            stdin: false,
        };
        hash_object
    }

    fn config(&mut self, args: &[String], output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let mut i = 0;
        while i < args.len() {
            i = self.add_setting(i, &args, output)?;
        }
        Ok(())
    }

    fn add_setting(
        &mut self,
        i: usize,
        args: &[String],
        output: &mut dyn Write,
    ) -> Result<usize, ErrorFlags> {
        let flags = [
            Self::add_type_config,
            Self::add_stdin_config,
            Self::add_file_config,
        ];
        for f in flags.iter() {
            match f(self, i, args, output) {
                Ok(i) => return Ok(i),
                Err(ErrorFlags::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ErrorFlags::InvalidArguments)
    }

    fn add_type_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
        output: &mut dyn Write,
    ) -> Result<usize, ErrorFlags> {
        if args[i] != "-t" {
            return Err(ErrorFlags::WrongFlag);
        }
        let object_type = args[i + 1].clone();
        if ![
            "blob".to_string(),
            "tree".to_string(),
            "commit".to_string(),
            "tag".to_string(),
        ]
        .contains(&object_type)
        {
            return Err(ErrorFlags::ObjectTypeError);
        }

        hash_object.object_type.to_string();
        Ok(i + 2)
    }

    fn add_stdin_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
        output: &mut dyn Write,
    ) -> Result<usize, ErrorFlags> {
        if args[i] != "--stdin" {
            return Err(ErrorFlags::WrongFlag);
        }
        hash_object.stdin = true;
        Ok(i + 1)
    }

    fn add_file_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
        output: &mut dyn Write,
    ) -> Result<usize, ErrorFlags> {
        if Self::is_flag(&args[i]) {
            return Err(ErrorFlags::WrongFlag);
        }
        if i < args.len() - 1 {
            return Err(ErrorFlags::InvalidArguments);
        }
        hash_object.file = Some(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let content: Vec<u8> = self.get_content(stdin)?;
        if content.is_empty() {
            return Ok(());
        }
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        writeln!(output, "{}", hex_string);
        Ok(())
    }

    fn get_content(&self, mut stdin: &mut dyn Read) -> Result<Vec<u8>, ErrorFlags> {
        if self.stdin {
            let mut input = String::new();
            stdin.read_to_string(&mut input);
            Ok(input.as_bytes().to_vec())
        } else {
            let Some(path) = &self.file else {
                // Return Vec<u8> empty
                return Ok(Vec::new());
            };
            read_file_contents(path)
        }
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

fn read_file_contents(path: &str) -> Result<Vec<u8>, ErrorFlags> {
    let mut file = File::open(path).map_err(|_| ErrorFlags::FileNotFound)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| ErrorFlags::FileReadError)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            HashObject::run_from("", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::CommandName)
        ));
    }

    #[test]
    fn test_path_null() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock).is_ok()
        );
        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        assert_eq!(output, "");
    }

    #[test]
    fn test_path() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &["./test/commands/hash_object/codigo1.txt".to_string()];
        assert!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock).is_ok()
        );

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock).is_ok()
        );

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type_error() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob2".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::ObjectTypeError)
        ));
    }

    #[test]
    fn test_value_before_flag() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "blob".to_string(),
            "-t".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    #[test]
    fn test_doubled_value_after_flag() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "blob".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    #[test]
    fn test_stdin() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &["--stdin".to_string()];
        assert!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock).is_ok()
        );

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }
}
