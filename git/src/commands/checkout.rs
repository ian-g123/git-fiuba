use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

pub struct Checkout {
    change_branch: String,
    new_branch: Vec<String>,
    update_paths: Vec<String>,
}

impl Command for Checkout {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "checkout" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![Self::add_change_branch_config]
    }
}

impl Checkout {
    /// Crea un comando Checkout. Devuelve error si el proceso de creación falla.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut checkout = Self::new_default();

        checkout.config(args)?;

        Ok(checkout)
    }

    fn new_default() -> Self {
        Self {
            change_branch: "".to_string(),
            new_branch: Vec::new(),
            update_paths: Vec::new(),
        }
    }

    /// Configura el flag para mostrar todas las ramas.
    fn add_change_branch_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        /* if !self.delete_locals.is_empty() || !self.delete_remotes.is_empty() {
            return Err(CommandError::ShowAllAndDelete);
        }

        if self.show_remotes == true {
            self.show_remotes = false;
        }
        self.show_all = true; */

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
