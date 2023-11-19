use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use super::command::check_errors_flags;

pub struct LsFiles {
    cached: bool,
    deleted: bool,
    modified: bool,
    others: bool,
    stage: bool,
    unmerged: bool,
    files: Vec<String>,
}

impl Command for LsFiles {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "ls-files" {
            return Err(CommandError::Name);
        }

        let mut instance = Self::new(args)?;
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

impl LsFiles {
    /// Crea un comando LsFiles. Devuelve error si el proceso de creación falla.
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

    /// Configura el flag -c.
    fn add_cached_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--cached".to_string(), "-c".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.cached = true;
        Ok(i + 1)
    }

    /// Configura el flag -d.
    fn add_deleted_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--deleted".to_string(), "-d".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.deleted = true;
        Ok(i + 1)
    }

    /// Configura el flag -m.
    fn add_modified_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--modified".to_string(), "-m".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.modified = true;
        Ok(i + 1)
    }

    /// Configura el flag -o.
    fn add_others_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--others".to_string(), "-o".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.others = true;
        Ok(i + 1)
    }

    /// Configura el flag -s.
    fn add_stage_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--stage".to_string(), "-s".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.stage = true;
        self.cached = true;
        Ok(i + 1)
    }

    /// Configura el flag -u.
    fn add_unmerged_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--unmerged".to_string(), "-u".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.unmerged = true;
        Ok(i + 1)
    }

    /// Configura los files a enlistar.
    fn add_files_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        self.files.push(args[i].clone());
        Ok(i + 1)
    }

    /// Ejecuta un comando LsFiles
    fn run(&mut self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;

        if !self.deleted && !self.others && !self.modified {
            self.cached = true;
        }
        repo.ls_files(
            self.cached,
            self.deleted,
            self.modified,
            self.others,
            self.stage,
            self.unmerged,
            self.files.clone(),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_invalid_name() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["".to_string()];
        match LsFiles::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::Name),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_invalid_arg() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-no".to_string()];
        match LsFiles::run_from("ls-files", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }
}
