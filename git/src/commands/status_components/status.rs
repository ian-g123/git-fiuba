use std::io::Read;
use std::io::Write;
use std::str;

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;
use crate::logger::Logger;

pub struct Status {
    branch: bool,
    short: bool,
}

impl Command for Status {
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), ErrorFlags> {
        if name != "status" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args, output)?;

        //instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, ErrorFlags>> {
        vec![Self::add_branch_config, Self::add_short_config]
    }
}

impl Status {
    /// Crea un comando Status. Devuelve error si el proceso de creación falla.
    fn new(args: &[String], output: &mut dyn Write) -> Result<Self, ErrorFlags> {
        if args.len() > 2 {
            //status -s -b (máximo)
            return Err(ErrorFlags::InvalidArguments);
        }
        let mut status = Self::new_default();

        status.config(args)?;

        Ok(status)
    }

    fn new_default() -> Self {
        Self {
            branch: false,
            short: false,
        }
    }

    /// Configura el flag 'branch'. Devuelve error si recibe argumentos o es un flag inválido.
    /// Caso contrario, devuelve el índice del próximo flag.
    fn add_branch_config(&mut self, i: usize, args: &[String]) -> Result<usize, ErrorFlags> {
        let options: Vec<String> = ["--branch".to_string(), "-b".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.branch = true;
        Ok(i + 1)
    }

    /// Configura el flag 'short'. Devuelve error si recibe argumentos o es un flag inválido.
    /// Caso contrario, devuelve el índice del próximo flag.
    fn add_short_config(&mut self, i: usize, args: &[String]) -> Result<usize, ErrorFlags> {
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.short = true;
        Ok(i + 1)
    }

    /// Comprueba si el flag es invalido. En ese caso, devuelve error.
    fn check_errors_flags(
        &self,
        i: usize,
        args: &[String],
        options: &[String],
    ) -> Result<(), ErrorFlags> {
        if !options.contains(&args[i]) {
            return Err(ErrorFlags::WrongFlag);
        }
        if i < args.len() - 1 && Self::is_flag(&args[i + 1]) {
            return Err(ErrorFlags::WrongFlag);
        }
        Ok(())
    }

    /* fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        write!(output, "{}", "");
        Ok(())
    } */
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use crate::logger::Logger;

    use super::*;

    /// Prueba que intentar crear un Status sin un nombre de comando, devuelve error.
    #[test]
    fn create_status_fails_no_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args: &[String] = &[];
        let logger = assert!(matches!(
            Status::run_from("", args, &mut stdin_mock, &mut stdout_mock, &mut logger),
            Err(ErrorFlags::CommandName)
        ));
    }

    /// Prueba que intentar crear un Status con un nombre distinto a 'status', devuelve error.
    #[test]
    fn create_status_fails_wrong_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from(
                "hash-object",
                args,
                &mut stdin_mock,
                &mut stdout_mock,
                &mut logger
            ),
            Err(ErrorFlags::CommandName)
        ));
    }

    /// Prueba que intentar crear un Status con más de 2 flags, devuelve error.
    #[test]
    fn create_status_fails_length() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args: &[String] = &[
            "-b".to_string(),
            "-s".to_string(),
            "tercer argumento".to_string(),
        ];
        assert!(matches!(
            Status::run_from(
                "status",
                args,
                &mut stdin_mock,
                &mut stdout_mock,
                &mut logger
            ),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    /// Prueba que se pueda crear un comando Status correctamente.
    #[test]
    fn create_status() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args: &[String] = &[];
        assert!(Status::run_from(
            "status",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());
    }

    /// Prueba que check_errors_flags() falla si se recibe un flag inválido.
    #[test]
    fn test_check_error_wrong_flag() {
        let i = 0;
        let args = ["no-existe".to_string()];
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let Ok(status_command) = Status::new(&args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = status_command.check_errors_flags(i, &args, &options);

        assert!(matches!(result, Err(ErrorFlags::WrongFlag)));
    }

    /// Prueba que check_errors_flags() falla si recibe values > 0.
    #[test]
    fn test_check_error_with_values() {
        let i: usize = 0;
        let args = ["-b".to_string(), "arg".to_string()];
        let values: Vec<String> = ["value".to_string()].to_vec();
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let Ok(status_command) = Status::new(&args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = status_command.check_errors_flags(i, &args, &options);

        assert!(matches!(result, Err(ErrorFlags::InvalidArguments)));
    }

    /// Prueba que check_errors_flags() no devuelve error si los argumentos son válidos.
    #[test]
    fn test_check_error() {
        let i: usize = 0;
        let args = ["-b".to_string()];
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let Ok(status_command) = Status::new(&args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = status_command.check_errors_flags(i, &args, &options);

        assert!(result.is_ok());
    }

    /// Prueba que add_short_config() falle si el flag no es 's' o 'short'.
    #[test]
    fn set_short_fails() {
        let i: usize = 0;
        let values: Vec<String> = Vec::new();

        let args: &[String] = &["-b".to_string()];

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_short_config(i, &values);
        assert!(result.is_err());
    }

    /// Prueba que add_short_config() funciones si el flag es 's'.
    #[test]
    fn add_short_config_s() {
        let i = 0;
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["-s".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_short_config(i, &values);
        assert!(result.is_ok());
    }

    /// Prueba que add_short_config() funcione si el flag es 'short'.
    #[test]
    fn add_short_config_short() {
        let i = 0;
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["--short".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_short_config(i, &values);
        assert!(result.is_ok());
    }

    /// Prueba que add_branch_config() falle si el flag no es 'b' o 'branch'.
    #[test]
    fn set_branch_fails() {
        let i = 0;
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["-s".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_branch_config(i, &values);
        assert!(result.is_err());
    }

    /// Prueba que add_branch_config() funciones si el flag es 'b'.
    #[test]
    fn add_branch_config_b() {
        let i = 0;
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["-b".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_branch_config(i, &values);
        assert!(result.is_ok());
    }

    /// Prueba que add_branch_config() funcione si el flag es 'branch'.
    #[test]
    fn add_branch_config_branch() {
        let i = 0;
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["--branch".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_branch_config(i, &values);
        assert!(result.is_ok());
    }
}
