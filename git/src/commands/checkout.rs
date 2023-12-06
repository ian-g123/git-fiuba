use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use super::command::check_errors_flags;

pub struct Checkout {
    new_branch: Vec<String>,
    checkout_or_update: Vec<String>,
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
        vec![
            Self::add_checkout_or_update_config,
            Self::add_new_branch_config,
        ]
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
            new_branch: Vec::new(),
            checkout_or_update: Vec::new(),
        }
    }

    /// Configura los files a actualizar o la rama a cambiar.
    fn add_checkout_or_update_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        self.checkout_or_update.push(args[i].clone());

        Ok(i + 1)
    }

    /// Configura la nueva rama a crear.
    fn add_new_branch_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["-b".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.checkout_or_update.push(args[i].clone());
        if args.len() - 1 == i {
            return Err(CommandError::SwitchRequiresValue);
        }
        if args.len() > 3 || self.checkout_or_update.len() > 1 {
            return Err(CommandError::UpdateAndSwicth(args[i + 1].clone()));
        }
        self.new_branch.push(args[i + 1].clone());
        if let Some(elem) = self.checkout_or_update.pop() {
            if !options.contains(&elem) {
                self.new_branch.push(elem);
            }
        }
        if i + 2 < args.len() {
            self.new_branch.push(args[i + 2].clone());
        } else if i > 0 {
            self.new_branch.push(args[0].clone());
        }

        Ok(args.len())
    }

    /// Ejecuta el comando Checkout
    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        if !self.checkout_or_update.is_empty() {
            repo.update_files_or_checkout(self.checkout_or_update.clone())?;
        } else if !self.new_branch.is_empty() {
            repo.create_branch_from_cmd_args(&self.new_branch)?;
            let name = self.new_branch[0].clone();
            repo.checkout(&name, true)?;
        } else {
            repo.show_tracking_info()?;
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
        match Checkout::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::Name),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_invalid_arg() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-no".to_string()];
        match Checkout::run_from("checkout", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_switch_no_value() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-b".to_string()];
        match Checkout::run_from("checkout", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::SwitchRequiresValue),
            Ok(_) => panic!(),
        }
    }
}
