use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use super::command::check_errors_flags;

pub struct LsTree {
    tree_ish: String,
    only_list_trees: bool,
    recursive: bool,
    show_tree_entries: bool,
    show_size: bool,
    only_name: bool,
}

impl Command for LsTree {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "ls-tree" {
            return Err(CommandError::Name);
        }

        let mut instance = Self::new(args)?;
        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![
            Self::add_tree_ish_config,
            Self::add_only_list_trees_config,
            Self::add_recursive_config,
            Self::add_show_tree_entries_config,
            Self::add_show_size_config,
            Self::add_only_name_config,
        ]
    }
}

impl LsTree {
    /// Crea una instancia del comando LsTree basándose en los argumentos recibidos.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut instance = Self::new_default();

        instance.config(args)?;

        Ok(instance)
    }

    /// Crea una instancia del comando LsTree con valores por default.
    fn new_default() -> Self {
        Self {
            tree_ish: "".to_string(),
            only_list_trees: false,
            recursive: false,
            show_tree_entries: false,
            show_size: false,
            only_name: false,
        }
    }

    /// Configura el argumento 'tree_ish'.
    fn add_tree_ish_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        self.tree_ish = args[i].clone();

        Ok(i + 1)
    }

    /// Configura el siguiente flag: '-d'.
    fn add_only_list_trees_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-d".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.only_list_trees = true;
        Ok(i + 1)
    }

    /// Configura el siguiente flag: '-r'.
    fn add_recursive_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-r".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.recursive = true;
        Ok(i + 1)
    }

    /// Configura el siguiente flag: '-t'.
    fn add_show_tree_entries_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-t".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.show_tree_entries = true;
        Ok(i + 1)
    }

    /// Configura los siguientes flags: '-l' y '--long'.
    fn add_show_size_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-l".to_string(), "--long".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.show_size = true;
        Ok(i + 1)
    }

    /// Configura los siguientes flags: '--name-only', '--status-only'.
    fn add_only_name_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> =
            ["--name-only".to_string(), "--status-only".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.only_name = true;
        Ok(i + 1)
    }

    /// Ejecuta el comando.
    fn run(&mut self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.ls_tree(
            &self.tree_ish,
            self.only_list_trees,
            self.recursive,
            self.show_tree_entries,
            self.show_size,
            self.only_name,
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
        match LsTree::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
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
        match LsTree::run_from("ls-tree", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }
}
