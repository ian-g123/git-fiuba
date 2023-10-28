use crate::commands::command::{Command, ConfigAdderFunction};
use git_lib::{
    command_errors::CommandError, git_repository::GitRepository, logger::Logger,
    objects::blob::Blob,
};
use std::{
    io::{Read, Write},
    str,
};

extern crate sha1;

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
        let mut repo = GitRepository::open("", output)?;
        if self.stdin {
            let mut input = String::new();
            if stdin.read_to_string(&mut input).is_ok() {
                let object = Blob::new_from_content_and_path(input.as_bytes().to_vec(), "")?;
                repo.hash_object(Box::new(object), self.write)?;
            };
        }
        for file in &self.files {
            let object = Blob::new_from_path(file.to_string())?;
            repo.hash_object(Box::new(object), self.write)?;
        }
        Ok(())
    }
}
