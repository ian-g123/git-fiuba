use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

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

    fn add_show_all_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--all".to_string(), "-a".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        if !self.delete_locals.is_empty() || !self.delete_remotes.is_empty() {
            return Err(CommandError::ShowAllAndDelete);
        }
        if self.show_remotes {
            self.show_remotes = false;
        }
        self.show_all = true;
        Ok(i + 1)
    }

    fn add_show_remotes_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--remotes".to_string(), "-r".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;

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

    fn add_delete_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-D".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        let mut branches: Vec<String> = Vec::new();
        let mut delete_remotes = false;
        for arg in 0..args.len() {
            if args[arg] == "-a" || args[arg] == "--all" {
                return Err(CommandError::ShowAllAndDelete);
            } else if args[arg] == "-m" {
                return Err(CommandError::RenameAndDelete);
            } else if args[arg] == "-r" || args[arg] == "--remotes" {
                delete_remotes = true;
            } else {
                branches.push(args[arg].clone())
            }
            // -D admite: branch names y -r
        }
        if delete_remotes {
            self.delete_remotes = branches;
        } else {
            self.delete_locals = branches;
        }

        Ok(args.len())
    }

    fn add_rename_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-m".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
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

    fn add_create_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        if args.len() > 3 {
            return Err(CommandError::FatalCreateBranchOperation);
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
                return Err(CommandError::WrongFlag);
            } else {
                branches_and_commits.push(args[arg].clone())
            }
        }
        self.create = branches_and_commits;

        Ok(args.len())
    }

    fn get_info(
        &self,
    ) -> (
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        bool,
        bool,
    ) {
        (
            self.rename.clone(),
            self.delete_locals.clone(),
            self.delete_remotes.clone(),
            self.create.clone(),
            self.show_remotes,
            self.show_all,
        )
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
        let (r, dl, dr, c, sa, sr) = self.get_info();

        let mut repo = GitRepository::open("", output)?;
        repo.log(&format!("Rename: {:?}, delete local: {:?}, delete remote: {:?}, create:  {:?}, show_all: {}, show_remotes: {}", r, dl, dr, c, sa, sr));
        if !self.rename.is_empty() {
            repo.rename_branch(&self.rename)?;
        } else if !self.create.is_empty() {
            repo.create_branch(&self.create)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /* #[test]
    fn test_() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-no".to_string()];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    } */
}
