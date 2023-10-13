use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::str;

extern crate sha1;
use sha1::{Digest, Sha1};

use crate::commands::command::Command;
use crate::commands::command_errors::CommandError;
use crate::logger::Logger;

/// Commando hash-object
pub struct HashObject {
    object_type: String,
    write: bool,
    files: Vec<String>,
    stdin: bool,
}

impl Command for HashObject {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "hash-object" {
            return Err(CommandError::Name.into());
        }

        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![
            Self::add_type_config,
            Self::add_stdin_config,
            Self::add_file_config,
            Self::add_write_config,
        ]
    }
}

impl HashObject {
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut hash_object = Self::new_default();
        hash_object.config(args)?;
        Ok(hash_object)
    }

    fn new_default() -> Self {
        Self {
            object_type: "blob".to_string(),
            files: Vec::<String>::new(),
            write: false,
            stdin: false,
        }
    }

    fn add_type_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if args[i] != "-t" {
            return Err(CommandError::WrongFlag);
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
            return Err(CommandError::ObjectTypeError);
        }

        hash_object.object_type.to_string();
        Ok(i + 2)
    }

    fn add_write_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if args[i] != "-w" {
            return Err(CommandError::WrongFlag);
        }
        hash_object.write = true;
        Ok(i + 1)
    }

    fn add_stdin_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if args[i] != "--stdin" {
            return Err(CommandError::WrongFlag);
        }
        hash_object.stdin = true;
        Ok(i + 1)
    }

    fn add_file_config(
        hash_object: &mut HashObject,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        hash_object.files.push(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        if self.stdin {
            let mut input = String::new();
            if stdin.read_to_string(&mut input).is_ok() {
                self.run_for_content(input.as_bytes().to_vec(), output)?;
            };
        }
        for file in &self.files {
            let content = read_file_contents(file)?;
            self.run_for_content(content, output)?;
        }
        Ok(())
    }

    fn run_for_content(
        &self,
        content: Vec<u8>,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        let _ = writeln!(output, "{}", hex_string);
        if self.write {
            let folder_name = &hex_string[0..2];
            let file_name = &hex_string[2..];

            let parent_path = format!(".git/objects/{}", folder_name);
            let path = format!("{}/{}", parent_path, file_name);
            if let Err(error) = fs::create_dir_all(parent_path) {
                return Err(CommandError::FileOpenError(error.to_string()));
            };
            let Ok(mut file) = File::create(path) else {
                return Err(CommandError::FileOpenError(
                    "Error al abrir archivo para escritura".to_string(),
                ));
            };
            file.write_all(&data);
        }
        Ok(())
    }

    fn get_header(&self, data: &Vec<u8>) -> String {
        let length = data.len();
        format!("{} {}\0", self.object_type, length)
    }

    fn get_sha1(&self, data: &[u8]) -> String {
        let mut sha1 = Sha1::new();
        sha1.update(data);
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

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::{env, fs, io::Cursor, path::Path};

    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        // test of returns error ErrorFlags::CommandName
        assert!(matches!(
            HashObject::run_from("", args, &mut stdin_mock, &mut stdout_mock, &mut logger),
            Err(CommandError::Name)
        ));
    }

    #[test]
    fn test_path_null() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());
        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        assert_eq!(output, "");
    }

    #[test]
    fn test_path() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &["./test/commands/hash_object/codigo1.txt".to_string()];
        assert!(HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type_error() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob2".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from(
                "hash-object",
                args,
                &mut stdin_mock,
                &mut stdout_mock,
                &mut logger
            ),
            Err(CommandError::ObjectTypeError)
        ));
    }

    #[test]
    fn test_value_before_flag() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "blob".to_string(),
            "-t".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from(
                "hash-object",
                args,
                &mut stdin_mock,
                &mut stdout_mock,
                &mut logger
            ),
            Err(CommandError::ObjectTypeError)
        ));
    }

    #[test]
    fn test_doubled_value_after_flag() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "blob".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];

        assert!(matches!(
            HashObject::run_from(
                "hash-object",
                args,
                &mut stdin_mock,
                &mut stdout_mock,
                &mut logger
            ),
            Err(CommandError::FileNotFound(_))
        ));
    }

    #[test]
    fn test_stdin() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &["--stdin".to_string()];
        assert!(HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_file_before_type() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./test/commands/hash_object/codigo1.txt".to_string(),
            "-t".to_string(),
            "blob".to_string(),
        ];
        match HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => {
                panic!("{error}")
            }
            Ok(_) => (),
        };

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_two_files() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./test/commands/hash_object/codigo1.txt".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        match HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => {
                panic!("{error}")
            }
            Ok(_) => (),
        };

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git =
            "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_two_files_and_stdin() {
        let mut logger = Logger::new("./test/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./test/commands/hash_object/codigo1.txt".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
            "--stdin".to_string(),
        ];
        match HashObject::run_from(
            "hash-object",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => {
                panic!("{error}")
            }
            Ok(_) => (),
        };

        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };

        // salida hexadecimal de git hash-object -t blob ./test/commands/hash_object/codigo1.txt
        let hex_git =
            "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }
}
