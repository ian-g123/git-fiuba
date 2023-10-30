use crate::commands::command::Command;
use git_lib::{command_errors::CommandError, git_repository::GitRepository, join_paths};

use std::{
    io::{Read, Write},
    vec,
};

pub struct CatFile {
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
    ) -> Result<(), CommandError> {
        if name != "cat-file" {
            return Err(CommandError::Name);
        }

        let mut cat_file = CatFile::new_default()?;
        cat_file.config(args)?;
        cat_file.run(output)
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![Self::add_configs]
    }
}

impl CatFile {
    fn new_default() -> Result<CatFile, CommandError> {
        let cat_file = CatFile {
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

        if !self.hash.is_empty() {
            return Err(CommandError::InvalidArguments);
        }

        self.hash = flag.to_string();
        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        {
            let mut repo = GitRepository::open("", output)?;
            if self.exists {
                if self.pretty || self.size || self.type_object {
                    return Err(CommandError::InvalidArguments);
                }
            } else if self.type_object {
                if self.pretty || self.size {
                    return Err(CommandError::InvalidArguments);
                } else {
                    repo.display_type_from_hash(&self.hash)?;
                }
            } else if self.size {
                if self.pretty {
                    return Err(CommandError::InvalidArguments);
                } else {
                    repo.display_size_from_hash(&self.hash)?;
                }
            } else if self.pretty {
                repo.display_from_hash(&self.hash)?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_invalid_name() {
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec![];
        let result = CatFile::run_from("cat-", &args, &mut stdin_mock, &mut stdout_mock);
        assert!(matches!(result, Err(CommandError::Name)))
    }

    #[test]
    fn test_not_enough_arguments() {
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-p".to_string()];
        let result = CatFile::run_from("cat-file", &args, &mut stdin_mock, &mut stdout_mock);
        assert!(matches!(result, Err(CommandError::NotEnoughArguments)))
    }
}
