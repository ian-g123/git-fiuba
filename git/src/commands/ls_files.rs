use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

pub struct Ls_files {
    cached: bool,
    deleted: bool,
    modified: bool,
    others: bool,
    stage: bool,
    unmerged: bool,
    files: Vec<String>,
}

impl Command for Ls_files {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "ls-files" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![
            Self::add_cached_config,
            Self::add_deleted_config,
            Self::add_modified_config,
            Self::add_others_config,
            Self::add_stage_config,
            Self::add_unmerged_config,
            Self::add_files_config,
        ]
    }
}

impl Ls_files {
    /// Crea un comando Ls_files. Devuelve error si el proceso de creación falla.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        if args.len() > 2 {
            return Err(CommandError::InvalidArguments);
        }
        let mut instance = Self::new_default();

        instance.config(args)?;

        Ok(instance)
    }

    fn new_default() -> Self {
        Self {
            cached: false,
            deleted: false,
            modified: false,
            others: false,
            stage: false,
            unmerged: false,
            files: Vec::new(),
        }
    }

    fn add_cached_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--cached".to_string(), "-c".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.cached = true;
        Ok(i + 1)
    }

    fn add_deleted_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--deleted".to_string(), "-d".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.deleted = true;
        Ok(i + 1)
    }

    fn add_modified_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--modified".to_string(), "-m".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.modified = true;
        Ok(i + 1)
    }

    fn add_others_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--others".to_string(), "-o".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.others = true;
        Ok(i + 1)
    }

    fn add_stage_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--stage".to_string(), "-s".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.stage = true;
        Ok(i + 1)
    }

    fn add_unmerged_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--unmerged".to_string(), "-u".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.unmerged = true;
        Ok(i + 1)
    }

    fn add_files_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        self.files.push(args[i].clone());
        Ok(i + 1)
    }

    /// Comprueba si el flag es invalido. En ese caso, devuelve error.
    fn check_errors_flags(
        i: usize,
        args: &[String],
        options: &[String],
    ) -> Result<(), CommandError> {
        if !options.contains(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        Ok(())
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        Ok(())
    }
}
