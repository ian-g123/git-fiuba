use std::fs::{self, create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use std::{env, str};

use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use crate::commands::command::{Command, ConfigAdderFunction};
use crate::logger::Logger;

/// Commando init
pub struct Init {
    branch_main: String,
    bare: bool,
    paths: Vec<String>,
}

impl Command for Init {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
        _logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "init" {
            return Err(CommandError::Name);
        }

        let mut instance = Self::new(args)?;

        if instance.paths.is_empty() {
            let current_dir = env::current_dir().map_err(|_| CommandError::InvalidArguments)?;
            let current_dir_display = current_dir.display();
            instance.paths.push(current_dir_display.to_string());
        }

        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![
            Self::add_bare_config,
            Self::add_branch_config,
            Self::add_path_config,
        ]
    }
}

impl Init {
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut init = Self::new_default();
        init.config(args)?;
        Ok(init)
    }

    fn new_default() -> Self {
        Self {
            branch_main: "master".to_string(),
            bare: false,
            paths: Vec::<String>::new(),
        }
    }

    fn add_bare_config(init: &mut Init, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--bare" {
            return Err(CommandError::WrongFlag);
        }
        init.bare = true;
        Ok(i + 1)
    }

    fn add_branch_config(
        init: &mut Init,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if args[i] != "-b" {
            return Err(CommandError::WrongFlag);
        }
        if args.len() <= i + 1 {
            return Err(CommandError::InvalidArguments);
        }

        init.branch_main = args[i + 1].clone();

        Ok(i + 2)
    }

    fn add_path_config(init: &mut Init, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        if !init.paths.is_empty() {
            return Err(CommandError::InvalidArguments);
        }
        let path_aux = args[i].clone();
        let _ = create_dir_all(&path_aux)
            .map_err(|error| CommandError::DirectoryCreationError(error.to_string()));
        let absolute_path_res = Path::new(&path_aux)
            .canonicalize()
            .map_err(|error| CommandError::DirNotFound(error.to_string()))?;

        let Some(absolute_path) = absolute_path_res.to_str() else {
            return Err(CommandError::DirNotFound(path_aux));
        };

        init.paths.push(absolute_path.to_string());

        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        for path in &self.paths {
            GitRepository::init(path, &self.branch_main, self.bare, output)?;
        }
        Ok(())
    }
}
