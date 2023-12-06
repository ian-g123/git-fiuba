use std::io;
use std::io::Read;
use std::io::Write;
use std::str;
use std::vec;

use crate::commands::command::Command;
use git_lib::command_errors::CommandError;
use git_lib::git_repository::GitRepository;

use super::command::check_errors_flags;

pub struct CheckIgnore {
    verbose: bool,
    stdin: bool,
    non_matching: bool,
    paths: Vec<String>,
}

impl Command for CheckIgnore {
    fn run_from(
        name: &str,
        args: &[String],
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "check-ignore" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output, input)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, CommandError>> {
        vec![
            Self::add_verbose_config,
            Self::add_non_matching_config,
            Self::add_stdin_config,
            Self::add_path_config,
        ]
    }
}

impl CheckIgnore {
    /// Crea un comando CheckIgnore. Devuelve error si el proceso de creación falla.
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut instance = Self::new_default();

        instance.config(args)?;

        Ok(instance)
    }

    fn new_default() -> Self {
        Self {
            verbose: false,
            stdin: false,
            non_matching: false,
            paths: Vec::new(),
        }
    }

    /// Configura el flag 'verbose'.
    fn add_verbose_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--verbose".to_string(), "-v".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.verbose = true;
        Ok(i + 1)
    }

    /// Configura el flag 'non-matching'.
    fn add_non_matching_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--non-matching".to_string(), "-n".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.non_matching = true;
        Ok(i + 1)
    }

    /// Configura el flag 'stdin'.
    fn add_stdin_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options: Vec<String> = ["--stdin".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        if !self.paths.is_empty() {
            return Err(CommandError::StdinAndPathsError);
        }
        self.stdin = true;
        Ok(i + 1)
    }

    /// Agrega paths al vector de paths del comando.
    fn add_path_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        if self.stdin {
            return Err(CommandError::StdinAndPathsError);
        }
        self.paths.push(args[i].clone());
        Ok(i + 1)
    }

    /// Ejecuta el comando CheckIgnore
    fn run(&self, output: &mut dyn Write, stdin: &mut dyn Read) -> Result<(), CommandError> {
        if !self.stdin && self.paths.is_empty() {
            return Err(CommandError::NoPathSpecified);
        }
        if self.non_matching && !self.verbose {
            return Err(CommandError::NonMatchingWithoutVerbose);
        }

        let mut repo = GitRepository::open("", output)?;

        if self.stdin {
            /* let reader = io::BufReader::new(input);

            for line in reader.lines() {
                if let Ok(line) = line {
                    repo.check_ignore_file(self.verbose, self.non_matching, &line, None)?;
                }
            } */
            let mut end = false;
            loop {
                let mut path = String::new();
                loop {
                    let mut buf = [0; 1];
                    if let Err(e) = stdin.read_exact(&mut buf) {
                        if e.kind() == io::ErrorKind::UnexpectedEof {
                            end = true;
                            let input = String::from_utf8_lossy(&buf).to_string();
                            path += &input;

                            break;
                        }

                        return Err(CommandError::StdinError);
                    };
                    let input = String::from_utf8_lossy(&buf).to_string();
                    if input == "\n" {
                        break;
                    }
                    path += &input;
                }

                if path.is_empty() {
                    return Err(CommandError::EmptyPath);
                }
                if path != "\0" {
                    repo.check_ignore_paths(self.verbose, self.non_matching, [path].as_ref())?;
                }

                if end {
                    break;
                }
            }
        } else {
            repo.check_ignore_paths(self.verbose, self.non_matching, &self.paths)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use super::*;

    /// Prueba que intentar crear un CheckIgnore sin un nombre de comando, devuelve error.
    #[test]
    fn create_check_ignore_fails_no_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            CheckIgnore::run_from("", args, &mut stdin_mock, &mut stdout_mock),
            Err(CommandError::Name)
        ));
    }

    /// Prueba que intentar crear un CheckIgnore con un nombre distinto a 'CheckIgnore', devuelve error.
    #[test]
    fn create_check_ignore_fails_wrong_command() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            CheckIgnore::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock,),
            Err(CommandError::Name)
        ));
    }

    /// Prueba que intentar crear un CheckIgnore con flags inválidos devuelve error, según la
    /// implementación de Command.
    #[test]
    fn new_check_ignore_fails_flag() {
        let args: &[String] = &["-b".to_string(), "-w".to_string()];

        assert!(matches!(
            CheckIgnore::new(args),
            Err(CommandError::InvalidArguments)
        ));
    }

    #[test]
    fn stdin_and_paths() {
        let args: &[String] = &["--stdin".to_string(), "path".to_string()];
        match CheckIgnore::new(args) {
            Ok(_) => assert!(false),
            Err(error) => assert_eq!(error, CommandError::StdinAndPathsError),
        }

        let args: &[String] = &["path".to_string(), "--stdin".to_string()];
        match CheckIgnore::new(args) {
            Ok(_) => assert!(false),
            Err(error) => assert_eq!(error, CommandError::StdinAndPathsError),
        }
    }

    #[test]
    fn no_paths() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &["-v".to_string()];
        assert!(matches!(
            CheckIgnore::run_from("check-ignore", args, &mut stdin_mock, &mut stdout_mock,),
            Err(CommandError::NoPathSpecified)
        ));
    }

    #[test]
    fn non_matching() {
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &["-n".to_string(), "path".to_string()];
        match CheckIgnore::run_from("check-ignore", args, &mut stdin_mock, &mut stdout_mock) {
            Ok(_) => assert!(false),
            Err(error) => assert_eq!(error, CommandError::NonMatchingWithoutVerbose),
        }
    }
}
