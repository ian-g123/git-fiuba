use std::io::Read;
use std::io::Write;
use std::str;

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;

pub struct Status {
    branch: bool,
    short: bool,
}

impl Command for Status {
    /// Corre un comando de tipo 'Status'. Devuelve error si falla la ejecución.
    fn run_from(
        name: &str,
        args: &[String],
        _: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        if name != "status" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args, output)?;

        //instance.run(stdin, output)?;
        Ok(())
    }
}

impl Status {
    /// Crea un comando Status. Devuelve error si el proceso de creación falla.
    fn new(args: &[String], output: &mut dyn Write) -> Result<Self, ErrorFlags> {
        if args.len() > 2 {
            //status -s -b (máximo)
            return Err(ErrorFlags::InvalidArguments);
        }
        let mut status = Status {
            branch: false,
            short: false,
        };

        status.config(args, output)?;

        Ok(status)
    }

    /// Configura los flags del comando. Devuelve error si hay flags o valores inválidos.
    fn config(&mut self, args: &[String], output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let mut current_flag = &String::new();
        let mut values_buffer = Vec::<String>::new();
        
        for arg in args {
            println!("{}", arg);
            if Self::is_flag(arg) {
                if !current_flag.is_empty() {
                    self.add_flag(current_flag, &values_buffer, output)?;
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }
        Ok(())
    }

    /// Agrega los flags al comando. Si hay flags o valores inválidos, devuelve error.
    fn add_flag(
        &mut self,
        flag: &str,
        values: &[String],
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        let flags = [Self::set_short_flag, Self::set_branch_flag];
        for f in flags.iter() {
            match f(self, flag, values, output) {
                Ok(_) => return Ok(()),
                Err(ErrorFlags::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ErrorFlags::WrongFlag)
    }

    /// Setea el flag 'branch'.
    fn set_branch_flag(
        &mut self,
        flag: &str,
        values: &[String],
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        let options: Vec<String> = ["--branch".to_string(), "-b".to_string()].to_vec();
        check_errors_flags(flag, values, &options)?;
        self.branch = true;
        Ok(())
    }

    /// Setea el flag 'short'.
    fn set_short_flag(
        &mut self,
        flag: &str,
        values: &[String],
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();
        check_errors_flags(flag, values, &options)?;
        self.short = true;
        Ok(())
    }

    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        write!(output, "{}", "");
        Ok(())
    }
}

/// Comprueba si el flag es invalido. En ese caso, devuelve error.
fn check_errors_flags(flag: &str, values: &[String], options: &[String]) -> Result<(), ErrorFlags> {
    if !options.contains(&flag.to_string()) {
        return Err(ErrorFlags::WrongFlag);
    }
    if !values.is_empty() {
        return Err(ErrorFlags::InvalidArguments);
    }
    Ok(())
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

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
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

        let args: &[String] = &[
            "-b".to_string(),
            "-s".to_string(),
            "tercer argumento".to_string(),
        ];
        assert!(matches!(
            Status::run_from("status", args, &mut stdin_mock, &mut stdout_mock),
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

        let args: &[String] = &[];
        assert!(Status::run_from("status", args, &mut stdin_mock, &mut stdout_mock).is_ok());
    }

    /// Prueba que check_errors_flags() falla si recibe un flag inválido.
    #[test]
    fn test_check_error_wrong_flag() {
        let flag = "no-existe";
        let values: Vec<String> = Vec::new();
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let result = check_errors_flags(flag, &values, &options);

        assert!(matches!(result, Err(ErrorFlags::WrongFlag)));
    }

    /// Prueba que check_errors_flags() falla si recibe values > 0.
    #[test]
    fn test_check_error_with_values() {
        let flag = "--short";
        let values: Vec<String> = ["value".to_string()].to_vec();
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let result = check_errors_flags(flag, &values, &options);

        assert!(matches!(result, Err(ErrorFlags::InvalidArguments)));
    }

    /// Prueba que check_errors_flags() no devuelve error si los argumentos son válidos.
    #[test]
    fn test_check_error() {
        let flag = "-s";
        let values: Vec<String> = Vec::new();
        let options: Vec<String> = ["--short".to_string(), "-s".to_string()].to_vec();

        let result = check_errors_flags(flag, &values, &options);

        assert!(result.is_ok());
    }

    /// Prueba que set_short_flag() falle si el flag no es 's' o 'short'.
    #[test]
    fn set_short_fails() {
        let flag: &str = "-b";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["-b".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_short_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_err());
    }

    /// Prueba que set_short_flag() funciones si el flag es 's'.
    #[test]
    fn set_short_flag_s() {
        let flag: &str = "-s";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);


        let args: &[String] = &["-s".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_short_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_ok());
    }

    /// Prueba que set_short_flag() funcione si el flag es 'short'.
    #[test]
    fn set_short_flag_short() {
        let flag: &str = "--short";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);


        let args: &[String] = &["--short".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_short_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_ok());
    }

    /// Prueba que set_branch_flag() falle si el flag no es 'b' o 'branch'.
    #[test]
    fn set_branch_fails() {
        let flag: &str = "-s";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);


        let args: &[String] = &["-s".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_branch_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_err());
    }

    /// Prueba que set_branch_flag() funciones si el flag es 'b'.
    #[test]
    fn set_branch_flag_b() {
        let flag: &str = "-b";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &["-b".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_branch_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_ok());
    }

    /// Prueba que set_branch_flag() funcione si el flag es 'branch'.
    #[test]
    fn set_branch_flag_branch() {
        let flag: &str = "--branch";
        let values: Vec<String> = Vec::new();
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);


        let args: &[String] = &["--branch".to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.set_branch_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_ok());
    }

    /// Prueba que el método add_flag() devuelva error si el flag es inválido.
    #[test]
    fn add_flag_fails() {
        let flag: &str = "no-existe";
        let values: Vec<String> = Vec::new();

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &[flag.to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_err());
    }

    /// Prueba que el método add_flag() funcione correctamente si el flag es válido.
    #[test]
    fn add_flag() {
        let flag: &str = "-b";
        let values: Vec<String> = Vec::new();

        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let args: &[String] = &[flag.to_string()];

        let Ok(mut command_status) = Status::new(args, &mut stdout_mock) else {
            assert!(false);
            return;
        };

        let result = command_status.add_flag(flag, &values, &mut stdout_mock);
        assert!(result.is_ok());
    }
}
