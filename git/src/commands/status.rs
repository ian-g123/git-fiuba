use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

pub struct Status {
    short: bool,
}

impl Command for Status {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "status" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![Self::add_short_config]
    }
}

impl Status {
    /// Crea un comando Status. Devuelve error si el proceso de creación falla.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        if args.len() > 2 {
            return Err(CommandError::InvalidArguments);
        }
        let mut status = Self::new_default();

        status.config(args)?;

        Ok(status)
    }

    fn new_default() -> Self {
        Self { short: false }
    }

    /// Configura el flag 'short'. Devuelve error si recibe argumentos o es un flag inválido.
    /// Caso contrario, devuelve el índice del próximo flag.
    fn add_short_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.short = true;
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
        if self.short {
            repo.status_short_format(false)
        } else {
            repo.status_long_format(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use super::*;

    /// Prueba que intentar crear un Status sin un nombre de comando, devuelve error.
    #[test]
    fn create_status_fails_no_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from("", args, &mut stdin_mock, &mut stdout_mock),
            Err(CommandError::Name)
        ));
    }

    /// Prueba que intentar crear un Status con un nombre distinto a 'status', devuelve error.
    #[test]
    fn create_status_fails_wrong_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock,),
            Err(CommandError::Name)
        ));
    }

    /// Prueba que intentar crear un Status con más de 2 argumentos devuelve error.
    #[test]
    fn create_status_fails_length() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[
            "-b".to_string(),
            "-s".to_string(),
            "tercer argumento".to_string(),
        ];
        assert!(matches!(
            Status::run_from("status", args, &mut stdin_mock, &mut stdout_mock,),
            Err(CommandError::InvalidArguments)
        ));
    }

    /// Prueba que intentar crear un Status con flags inválidos devuelve error, según la
    /// implementación de Command.
    #[test]
    fn new_status_fails_flag() {
        let args: &[String] = &["-b".to_string(), "-w".to_string()];

        assert!(matches!(
            Status::new(args),
            Err(CommandError::InvalidArguments)
        ));
    }

    /// Prueba que check_errors_flags() falla si se recibe un flag inválido.
    #[test]
    fn test_check_error_wrong_flag() {
        let i = 0;
        let args = ["no-existe".to_string()];
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let result = Status::check_errors_flags(i, &args, &options);

        assert!(matches!(result, Err(CommandError::WrongFlag)));
    }

    /// Prueba que check_errors_flags() no devuelve error si los argumentos son válidos.
    #[test]
    fn test_check_error() {
        let i: usize = 0;
        let args = ["-b".to_string()];
        let options: Vec<String> = ["--branch".to_string(), "-b".to_string()].to_vec();

        let result = Status::check_errors_flags(i, &args, &options);

        assert!(result.is_ok());
    }
}
