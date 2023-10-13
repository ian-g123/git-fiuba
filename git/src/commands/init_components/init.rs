use std::fs::{self, File};
use std::io::{Read, Write};
use std::{env, str};

use crate::commands::command::{Command, ConfigAdderFunction};
use crate::commands::command_errors::CommandError;
use crate::logger::Logger;

/// Commando init
pub struct Init {
    branch_main: String,
    working_directory: bool,
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
            branch_main: "main".to_string(),
            working_directory: true,
            paths: Vec::<String>::new(),
        }
    }

    fn add_bare_config(init: &mut Init, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--bare" {
            return Err(CommandError::WrongFlag);
        }
        init.working_directory = false;
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
        let path_aux = args[i].clone();
        let root = if path_aux.starts_with('/') {
            path_aux
        } else {
            let current_dir = env::current_dir().map_err(|_| CommandError::InvalidArguments)?;
            let current_dir_display = current_dir.display();
            format!("{}/{}", current_dir_display, path_aux)
        };
        init.paths.push(root);

        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        for path in &self.paths {
            self.run_for_content(path, output)?;
        }
        Ok(())
    }

    fn run_for_content(&self, path: &String, output: &mut dyn Write) -> Result<(), CommandError> {
        self.create_dirs(path)?;
        self.create_files(path)?;
        let output_text = format!("Initialized empty Git repository in {}", path);
        let _ = writeln!(output, "{}", output_text);
        Ok(())
    }

    fn create_dirs(&self, path: &String) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_err() {
            return Err(CommandError::InvalidArguments);
        }
        let path_aux = if !self.working_directory {
            path.clone()
        } else {
            format!("{}/.git", path)
        };
        self.create_dir(&path_aux, "objects".to_string())?;
        self.create_dir(&path_aux, "objects/info".to_string())?;
        self.create_dir(&path_aux, "objects/pack".to_string())?;
        self.create_dir(&path_aux, "refs".to_string())?;
        self.create_dir(&path_aux, "refs/tags".to_string())?;
        self.create_dir(&path_aux, "refs/heads".to_string())?;
        self.create_dir(&path_aux, "branches".to_string())?;
        Ok(())
    }

    fn create_dir(&self, file: &String, name: String) -> Result<(), CommandError> {
        if fs::create_dir_all(format!("{}/{}", file, name)).is_ok() {
            Ok(())
        } else {
            Err(CommandError::InvalidArguments)
        }
    }

    fn create_files(&self, path: &String) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_err() {
            return Err(CommandError::InvalidArguments);
        }
        let path_aux = if !self.working_directory {
            path.clone()
        } else {
            format!("{}/.git", path)
        };
        self.create_file(&path_aux, "HEAD".to_string())?;
        Ok(())
    }

    fn create_file(&self, path: &String, name: String) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_ok() {
            match File::create(format!("{}/{}", path, name)) {
                Ok(mut archivo) => {
                    let texto = format!("ref: refs/heads/{}", self.branch_main);
                    let _: Result<(), CommandError> = match archivo.write_all(texto.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(CommandError::InvalidArguments),
                    };
                }
                Err(_) => return Err(CommandError::InvalidArguments),
            };
        } else {
            return Err(CommandError::InvalidArguments);
        }

        Ok(())
    }
}
