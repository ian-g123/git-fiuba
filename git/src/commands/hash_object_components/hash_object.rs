use std::{
    fs::{self, File},
    io::{Read, Write},
    str,
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        file_compressor::compress,
        objects::{aux::u8_vec_to_hex_string, blob::Blob, git_object::GitObjectTrait},
        objects_database,
    },
    logger::Logger,
};

extern crate sha1;
use sha1::{Digest, Sha1};

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
            return Err(CommandError::Name);
        }

        let instance = Self::new_from(args)?;

        logger.log(&format!("hash-object {:?}", args));
        instance.run(stdin, output, logger)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![
            Self::add_type_config,
            Self::add_stdin_config,
            Self::add_write_config,
            Self::add_file_config,
        ]
    }
}

impl HashObject {
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut hash_object = Self::new_default();
        hash_object.config(args)?;
        Ok(hash_object)
    }

    fn new_default() -> Self {
        Self::new("blob".to_string(), Vec::<String>::new(), false, false)
    }

    /// Instancia un nuevo HashObject
    pub fn new(object_type: String, files: Vec<String>, write: bool, stdin: bool) -> Self {
        Self {
            object_type,
            files,
            write,
            stdin,
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
        hash_object.files.push(args[i].clone());
        Ok(i + 1)
    }

    fn run(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if self.stdin {
            let mut input = String::new();
            if stdin.read_to_string(&mut input).is_ok() {
                let object = Blob::new_from_content(input.as_bytes().to_vec())?;
                self.hash_object(object, output, logger)?;
            };
        }
        for file in &self.files {
            let object = Blob::new_from_path(file.to_string())?;
            self.hash_object(object, output, logger)?;
        }
        Ok(())
    }

    fn hash_object(
        &self,
        object: Blob,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let hex_string = u8_vec_to_hex_string(&object.get_hash()?);
        if self.write {
            objects_database::write(Box::new(object))?;
        }
        let _ = writeln!(output, "{}", hex_string);
        logger.log(&format!("Writen object to database in {:?}", hex_string));
        Ok(())
    }

    /// Ejecuta el comando hash-object para el contenido de un archivo y devuelve el hash
    /// hexadecimal del contenido
    pub fn run_for_content(
        &self,
        content: Vec<u8>,
    ) -> Result<(String, Option<String>), CommandError> {
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        if self.write {
            let folder_name = &hex_string[0..2];
            let file_name = &hex_string[2..];

            let parent_path = format!(".git/objects/{}", folder_name);
            let path = format!("{}/{}", parent_path, file_name);
            if let Err(error) = fs::create_dir_all(parent_path) {
                return Err(CommandError::FileOpenError(error.to_string()));
            };
            let Ok(mut file) = File::create(&path) else {
                return Err(CommandError::FileOpenError(
                    "Error al abrir archivo para escritura".to_string(),
                ));
            };
            let compressed_data = compress(&data)?;
            if let Err(error) = file.write_all(&compressed_data) {
                return Err(CommandError::FileWriteError(error.to_string()));
            };
            return Ok((hex_string, Some(path)));
        }
        Ok((hex_string, None))
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
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &["./tests/commands/hash_object/codigo1.txt".to_string()];
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

        // salida hexadecimal de git hash-object ./tests/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type() {
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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

        // salida hexadecimal de git hash-object -t blob ./tests/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type_error() {
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob2".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "blob".to_string(),
            "-t".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "blob".to_string(),
            "blob".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
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
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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

        // salida hexadecimal de git hash-object -t blob ./tests/commands/hash_object/codigo1.txt
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_two_files() {
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./tests/commands/hash_object/codigo1.txt".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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

        // salida hexadecimal de git hash-object -t blob ./tests/commands/hash_object/codigo1.txt
        let hex_git =
            "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_two_files_and_stdin() {
        let mut logger = Logger::new("./tests/commands/hash_object/codigo1/.git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "./tests/commands/hash_object/codigo1.txt".to_string(),
            "./tests/commands/hash_object/codigo1.txt".to_string(),
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

        // salida hexadecimal de git hash-object -t blob ./tests/commands/hash_object/codigo1.txt
        let hex_git =
            "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\ne31f3beeeedd1a034c5ce6f1b3b2d03f02541d59\n";
        assert_eq!(output, hex_git);
    }
}
