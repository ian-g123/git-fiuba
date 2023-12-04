use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use super::command::check_errors_flags;

pub struct Branch {
    rename: Vec<String>,
    delete_locals: Vec<String>,
    delete_remotes: Vec<String>,
    create: Vec<String>,
    show_remotes: bool,
    show_all: bool,
}

impl Command for Branch {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "branch" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![
            Self::add_rename_config,
            Self::add_delete_config,
            Self::add_create_config,
            Self::add_show_remotes_config,
            Self::add_show_all_config,
        ]
    }
}

impl Branch {
    /// Crea un comando Branch. Devuelve error si el proceso de creación falla.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut branch = Self::new_default();

        branch.config(args)?;

        Ok(branch)
    }

    fn new_default() -> Self {
        Self {
            rename: Vec::new(),
            delete_locals: Vec::new(),
            delete_remotes: Vec::new(),
            create: Vec::new(),
            show_remotes: false,
            show_all: false,
        }
    }

    /// Configura el flag para mostrar todas las ramas.
    fn add_show_all_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--all".to_string(), "-a".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        if !self.delete_locals.is_empty() || !self.delete_remotes.is_empty() {
            return Err(CommandError::ShowAllAndDelete);
        }

        if self.show_remotes == true {
            self.show_remotes = false;
        }
        self.show_all = true;

        Ok(i + 1)
    }

    /// Configura el flag para mostrar las ramas remotas.
    fn add_show_remotes_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--remotes".to_string(), "-r".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;

        if !self.delete_locals.is_empty() {
            self.delete_remotes.append(&mut self.delete_locals);
            self.delete_locals.clear();
        }
        if self.show_all {
            self.show_all = false;
        }
        self.show_remotes = true;

        Ok(i + 1)
    }

    /// Configura el flag para eliminar ramas.
    fn add_delete_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-D".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        let mut branches: Vec<String> = Vec::new();
        let mut delete_remotes = false;
        for arg in 0..args.len() {
            if args[arg] == "-a" || args[arg] == "--all" {
                return Err(CommandError::ShowAllAndDelete);
            } else if args[arg] == "-m" {
                return Err(CommandError::RenameAndDelete);
            } else if args[arg] == "-r" || args[arg] == "--remotes" {
                delete_remotes = true;
            } else if args[arg] != "-D" {
                branches.push(args[arg].clone())
            }
            // -D admite: branch names y -r
        }
        if delete_remotes {
            self.delete_remotes = branches.clone();
        } else {
            self.delete_locals = branches.clone();
        }
        if branches.is_empty() {
            return Err(CommandError::DeleteWithNoArgs);
        }
        Ok(args.len())
    }

    /// Configura el flag para renombrar ramas.
    fn add_rename_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-m".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        if args.len() > 3 {
            return Err(CommandError::FatalRenameOperation);
        }
        self.show_all = false;
        self.show_remotes = false;
        let mut names: Vec<String> = Vec::new();
        for arg in 0..args.len() {
            if args[arg] == "-D" {
                return Err(CommandError::RenameAndDelete);
            } else if !Self::is_flag(&args[arg]) {
                names.push(args[arg].clone())
            }
        }
        self.rename = names;

        Ok(args.len())
    }

    /// Configura el flag para crear ramas.
    fn add_create_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        let mut branches_and_commits: Vec<String> = Vec::new();
        for arg in 0..args.len() {
            if args[arg] == "-a"
                || args[arg] == "--all"
                || args[arg] == "-r"
                || args[arg] == "--remotes"
            {
                return Err(CommandError::CreateAndListError);
            } else if args[arg] == "-D" {
                return Ok(arg);
            } else {
                branches_and_commits.push(args[arg].clone())
            }
        }

        if branches_and_commits.len() > 2 {
            return Err(CommandError::FatalCreateBranchOperation);
        }

        self.create = branches_and_commits;
        Ok(args.len())
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        if !self.rename.is_empty() {
            repo.rename_branch(&self.rename)?;
        } else if !self.create.is_empty() {
            repo.create_branch(&self.create, None)?;
        } else if !self.delete_locals.is_empty() {
            repo.delete_branches(&self.delete_locals, false)?;
        } else if !self.delete_remotes.is_empty() {
            repo.delete_branches(&self.delete_remotes, true)?;
        } else if self.show_all {
            repo.show_all_branches()?;
        } else if self.show_remotes {
            repo.show_remote_branches()?;
        } else {
            repo.show_local_branches()?;
        }

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
        match Branch::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
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
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_create_and_list() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["--remotes".to_string(), "new_branch".to_string()];
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::CreateAndListError),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_rename_and_delete() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-D".to_string(), "-m".to_string(), "new_branch".to_string()];
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::RenameAndDelete),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_create_too_many_args() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = [
            "new_branch".to_string(),
            "base".to_string(),
            "third".to_string(),
        ];
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::FatalCreateBranchOperation),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_delete_with_no_args() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-D".to_string()];
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::DeleteWithNoArgs),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_show_all_and_delete() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-D".to_string(), "--all".to_string()];
        match Branch::run_from("branch", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::ShowAllAndDelete),
            Ok(_) => assert!(false),
        }
    }
}
