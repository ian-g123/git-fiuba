use std::fs::File;
use std::io::{Read, Write};
use std::str;

extern crate sha1;

use sha1::{Digest, Sha1};

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;

pub struct Init {
    root : String,
    branch_main: String,
    working_directory: bool,
}

impl Command for Init {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        if name != "init" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }
}

impl Init {
    fn new(args: &[String]) -> Result<Self, ErrorFlags> {
        let aux = args
            .first()
            .map_or("".to_string(), |first_arg| {
        if !Self::is_flag(first_arg) {
            first_arg.to_string()
        } else {
            "".to_string()
        }
    });


        let mut init = Init {
            root : "".to_string(),
            branch_main: "main".to_string(),
            working_directory : true,
        };

        init.config(args)?;

        Ok(init)
    }

    fn config(&mut self, args: &[String]) -> Result<(), ErrorFlags> {
        let mut current_flag = "";
        let mut values_buffer = Vec::<String>::new();

        for arg in args {
            if Self::is_flag(&arg) {
                if !current_flag.is_empty() {
                    self.add_flag(current_flag, &values_buffer)?;
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }
        Ok(())
    }

    fn is_flag(arg: &str) -> bool {
        arg.starts_with("-")
    }

    fn add_flag(
        &mut self,
        flag: &str,
        values: &Vec<String>
    ) -> Result<(), ErrorFlags> {
        let flags = [Self::add_bare_flag];
        for f in flags.iter() {
            match f(self, flag, values) {
                Ok(_) => return Ok(()),
                Err(ErrorFlags::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ErrorFlags::WrongFlag)
    }

    fn add_bare_flag(
        init: &mut Init,
        flag: &str,
        values: &Vec<String>,
    ) -> Result<(), ErrorFlags> {
        if flag != "--bare" {
            return Err(ErrorFlags::WrongFlag);
        }
        if values.len() > 1 {
            return Err(ErrorFlags::ObjectTypeError);
        }
        if values.len() == 1 {
            init.root = values[0]
        }
        init.working_directory = false;
        Ok(())
    }

    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let content: Vec<u8> = self.get_content(stdin)?;
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        write!(output, "{}", hex_string);
        Ok(())
    }

    fn get_content(&self, mut stdin: &mut dyn Read) -> Result<Vec<u8>, ErrorFlags> {
        if self.stdin {
            let mut input = String::new();
            stdin.read_to_string(&mut input);
            Ok(input.as_bytes().to_vec())
        } else {
            read_file_contents(&self.path)
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
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
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

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    #[test]
    fn test_path() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &["./test/commands/hash_object/codigo1.txt".to_string()];
        assert!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock).is_ok()
        );

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
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
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
        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59";
        assert_eq!(output, hex_git);
    }

    #[test]
    fn test_object_type_error() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
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
    #[ignore]
    fn test_object_type_tree_error() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let args: &[String] = &[
            "-t".to_string(),
            "tree".to_string(),
            "./test/commands/hash_object/codigo1.txt".to_string(),
        ];
        assert!(matches!(
            HashObject::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::ObjectTypeError)
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

        let hex_git = "e31f3beeeedd1a034c5ce6f1b3b2d03f02541d59";
        assert_eq!(output, hex_git);
    }
}
