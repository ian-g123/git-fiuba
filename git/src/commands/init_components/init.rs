use std::fs::{self, create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use std::{env, str};

use crate::commands::command::{Command, ConfigAdderFunction};
use crate::commands::command_errors::CommandError;
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
            bare: true,
            paths: Vec::<String>::new(),
        }
    }

    fn add_bare_config(init: &mut Init, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--bare" {
            return Err(CommandError::WrongFlag);
        }
        init.bare = false;
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
            return Err(CommandError::DirectoryCreationError(path.clone()));
        }
        let git_path = if !self.bare {
            path.clone()
        } else {
            format!("{}/.git", path)
        };
        self.create_dir(&git_path, "objects".to_string())?;
        self.create_dir(&git_path, "objects/info".to_string())?;
        self.create_dir(&git_path, "objects/pack".to_string())?;
        self.create_dir(&git_path, "refs".to_string())?;
        self.create_dir(&git_path, "refs/tags".to_string())?;
        self.create_dir(&git_path, "refs/heads".to_string())?;
        self.create_dir(&git_path, "branches".to_string())?;
        Ok(())
    }

    fn create_dir(&self, path: &String, name: String) -> Result<(), CommandError> {
        let path_complete = format!("{}/{}", path, name);
        if fs::create_dir_all(&path_complete).is_ok() {
            Ok(())
        } else {
            Err(CommandError::DirectoryCreationError(path_complete))
        }
    }

    fn create_files(&self, path: &String) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_err() {
            return Err(CommandError::DirectoryCreationError(path.clone()));
        }
        let path_aux = if !self.bare {
            path.clone()
        } else {
            format!("{}/.git", path)
        };
        self.create_file(&path_aux, "HEAD".to_string())?;
        Ok(())
    }

    fn create_file(&self, path: &String, name: String) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_ok() {
            let path_complete = format!("{}/{}", path, name);
            match File::create(&path_complete) {
                Ok(mut archivo) => {
                    let texto = format!("ref: refs/heads/{}", self.branch_main);
                    let _: Result<(), CommandError> = match archivo.write_all(texto.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(CommandError::FileWriteError(err.to_string())),
                    };
                }
                Err(err) => return Err(CommandError::FileCreationError(err.to_string())),
            };
        } else {
            return Err(CommandError::DirectoryCreationError(path.clone()));
        }
        Ok(())
    }
}
