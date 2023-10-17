use chrono::format;

use crate::{
    commands::{
        command::Command,
        command_errors::CommandError,
        file_compressor::extract,
        objects::git_object::{self, GitObject},
        objects_database,
    },
    logger::Logger,
};

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    vec,
};

extern crate libflate;

pub struct CatFile {
    path: String,
    hash: String,
    exists: bool,
    pretty: bool,
    type_object: bool,
    size: bool,
}

fn is_flag(arg: &str) -> bool {
    if arg.starts_with('-') && arg.len() == 2 {
        let flag = match arg.chars().nth(1).ok_or(CommandError::WrongFlag) {
            Ok(flag) => flag,
            Err(_) => return false,
        };

        if ['p', 't', 's', 'e'].contains(&flag) {
            return true;
        }
    }
    false
}

impl Command for CatFile {
    fn run_from(
        name: &str,
        args: &[String],
        _input: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "cat-file" {
            return Err(CommandError::Name);
        }
        logger.log(&format!("cat-file args: {:?}", args));

        let mut cat_file = CatFile::new_default(args)?;
        cat_file.config(args)?;
        cat_file.run(output, logger)
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![Self::add_configs]
    }
}

impl CatFile {
    fn new_default(args: &[String]) -> Result<CatFile, CommandError> {
        let mut cat_file = CatFile {
            path: "".to_string(),
            exists: false,
            pretty: false,
            type_object: false,
            size: false,
            hash: "".to_string(),
        };

        Ok(cat_file)
    }

    fn add_configs(self: &mut CatFile, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args.len() < 2 {
            return Err(CommandError::NotEnoughArguments);
        }

        let flag = args[i].as_str();

        if is_flag(flag) {
            match flag {
                "-p" => self.pretty = true,
                "-t" => self.type_object = true,
                "-s" => self.size = true,
                "-e" => self.exists = true,
                _ => return Err(CommandError::WrongFlag),
            }
            return Ok(i + 1);
        }

        if !self.path.is_empty() {
            return Err(CommandError::InvalidArguments);
        }

        self.path = format!(".git/objects/{}/{}", &flag[..2], &flag[2..]);
        self.hash = flag.to_string();
        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write, logger: &mut Logger) -> Result<(), CommandError> {
        // let object = objects_database::read_from_path(&self.path, logger)?;
        // let object: GitObject = objects_database::read(&self.hash, logger)?;
        // logger.log("got object");
        // logger.log(&format!("object: {:?}", object));
        // let path = Path::new(&self.path);

        // if !path.exists() {
        //     return Err(CommandError::FileNotFound(self.path.clone()));
        // }

        // let data = obtain_data(path)?;

        // let (header, content) = match data.split_once('\0') {
        //     Some((header, content)) => (header, content),
        //     None => return Err(CommandError::ObjectTypeError),
        // };

        // let (object_type, size) = match header.split_once(' ') {
        //     Some((object_type, size)) => (object_type, size),
        //     None => return Err(CommandError::ObjectTypeError),
        // };
        self.show_in_output_bis(output, logger)
        // match self.show_in_output(output, object_type, size, content) {
        //     Ok(()) => {}
        //     Err(error) => return Err(error),
        // };
        // Ok(())
    }

    // fn show_in_output(
    //     &self,
    //     output: &mut dyn Write,
    //     object_type: &str,
    //     size: &str,
    //     content: &str,
    // ) -> Result<(), CommandError> {
    //     if self.exists {
    //         if self.pretty || self.size || self.type_object {
    //             return Err(CommandError::InvalidArguments);
    //         }
    //     } else if self.type_object {
    //         if self.pretty || self.size {
    //             return Err(CommandError::InvalidArguments);
    //         } else {
    //             _ = writeln!(output, "{}", object_type);
    //         }
    //     } else if self.size {
    //         if self.pretty {
    //             return Err(CommandError::InvalidArguments);
    //         } else {
    //             _ = writeln!(output, "{}", size);
    //         }
    //     } else if self.pretty {
    //         _ = writeln!(output, "{}", content);
    //     }

    //     Ok(())
    // }

    fn show_in_output_bis(
        &self,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if self.exists {
            if self.pretty || self.size || self.type_object {
                return Err(CommandError::InvalidArguments);
            }
        } else if self.type_object {
            if self.pretty || self.size {
                return Err(CommandError::InvalidArguments);
            } else {
                git_object::display_type_from_hash(output, &self.hash, logger)?;
            }
        } else if self.size {
            if self.pretty {
                return Err(CommandError::InvalidArguments);
            } else {
                git_object::display_size_from_hash(output, &self.hash, logger)?;
            }
        } else if self.pretty {
            git_object::display_from_hash(output, &self.hash, logger)?;
        } else {
            logger.log("wtf");
        }

        Ok(())
    }
}

/// Obtiene toda la data del archivo comprimido
fn obtain_data(path: &Path) -> Result<String, CommandError> {
    let string_path = match path.to_str() {
        Some(string_path) => string_path.to_string(),
        None => return Err(CommandError::InvalidFileName),
    };

    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(string_path.clone()))?;
    let mut data = Vec::new();

    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileNotFound(string_path))?;

    let data = extract(&data)?;
    let data = String::from_utf8(data).map_err(|_| CommandError::ObjectTypeError)?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_invalid_name() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec![];
        let result = CatFile::run_from(
            "cat-",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(CommandError::Name)))
    }

    #[test]
    fn test_not_enough_arguments() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-p".to_string()];
        let result = CatFile::run_from(
            "cat-file",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(CommandError::NotEnoughArguments)))
    }
}
