use std::{io::Read, io::Write};

use crate::commands::command::{Command, ConfigAdderFunction};
use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::check_errors_flags;

/// Hace referencia a un Comando Commit.
pub struct Commit {
    all: bool,
    reuse_message: Option<String>,
    dry_run: bool,
    message: Option<String>,
    quiet: bool,
    files: Vec<String>,
}

impl Command for Commit {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "commit" {
            return Err(CommandError::Name);
        }

        let instance = Commit::new_from(args)?;

        instance.run(stdin, output)?;

        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Commit> {
        vec![
            Commit::add_all_config,
            Commit::add_dry_run_config,
            Commit::add_message_config,
            Commit::add_quiet_config,
            Commit::add_reuse_message_config,
            Commit::add_pathspec_config,
        ]
    }
}

impl Commit {
    /// Crea un nuevo Comando Commit a partir de sus argumentos. Lo configura.
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut commit = Self::new_default();
        commit.config(args)?;
        Ok(commit)
    }

    /// Crea un nuevo Comando Commit a partir de valores por default.
    fn new_default() -> Self {
        Commit {
            all: false,
            reuse_message: None,
            dry_run: false,
            message: None,
            quiet: false,
            files: Vec::new(),
        }
    }

    /// Configura el flag -C.
    fn add_reuse_message_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options = ["-C".to_string(), "--reuse-message".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::ReuseMessageNoValue)?;
        self.reuse_message = Some(args[i + 1].clone());
        Ok(i + 2)
    }

    /// Configura el flag -m.
    fn add_message_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-m".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::MessageNoValue)?;
        let mut new_message: String = String::new();
        if let Some(message) = &self.message {
            new_message = format!("{}\n\n", message)
        }
        let (message, words) = read_message_completely(i, args)?;
        new_message += &message;
        self.message = Some(new_message);
        Ok(i + words + 1)
    }

    /// Configura el flag --dry-run.
    fn add_dry_run_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--dry-run".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.dry_run = true;
        Ok(i + 1)
    }

    /// Configura el flag -q.
    fn add_quiet_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-q".to_string(), "--quiet".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.quiet = true;
        Ok(i + 1)
    }

    /// Configura el flag (--all | -a).
    fn add_all_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-a".to_string(), "--all".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.all = true;
        Ok(i + 1)
    }

    /// Configura un Commit que recibe paths para commitear.
    fn add_pathspec_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::InvalidArguments);
        }
        self.files.push(args[i].clone());
        Ok(i + 1)
    }

    /// Ejecuta el Comando Commit.
    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        if self.message.is_some() && self.reuse_message.is_some() {
            return Err(CommandError::MessageAndReuseError);
        }

        let message = self.get_commit_message(stdin)?;

        if !self.files.is_empty() && self.all {
            return Err(CommandError::AllAndFilesFlagsCombination(
                self.files[0].clone(),
            ));
        }

        let mut repo = GitRepository::open("", output)?;
        if !self.files.is_empty() {
            repo.commit_files(
                message,
                &self.files,
                self.dry_run,
                self.reuse_message.clone(),
                self.quiet,
            )
        } else if self.all {
            repo.commit_all(
                message,
                &self.files,
                self.dry_run,
                self.reuse_message.clone(),
                self.quiet,
            )
        } else {
            repo.commit(
                message,
                &self.files,
                self.dry_run,
                self.reuse_message.clone(),
                self.quiet,
            )
        }
    }

    /// Devuelve el mensage del Commit. Si se usó el flag -m, devuelve el mensaje asociado.\
    /// Si hay que reusar el de otro commit (-C), devuelve un string vacío.\
    /// Si no se ha usado ninguno de esos flags, se pide al usuario que introduzca el mensaje nuevamente.
    fn get_commit_message(&self, stdin: &mut dyn Read) -> Result<String, CommandError> {
        let message = {
            if let Some(message) = self.message.clone() {
                message
            } else if self.reuse_message.is_some() {
                "".to_string()
            } else {
                let stdout: String = get_enter_message_text()?;
                run_enter_message(stdin, stdout)?
            }
        };
        Ok(message)
    }
}

/// Lee el mensaje introducido por el usuario por entrada estandar.
pub fn run_enter_message(stdin: &mut dyn Read, stdout: String) -> Result<String, CommandError> {
    println!("{}#\n", stdout);
    let mut message = read_from_stdin(stdin)?;
    message = ignore_commented_lines(message);

    if message.is_empty() {
        return Err(CommandError::CommitMessageEmptyValue);
    }

    Ok(message.trim().to_string())
}

/// Devuelve el texto que se mostrará si el Cliente no ha introducido un mensaje para el Commit.
fn get_enter_message_text() -> Result<String, CommandError> {
    let mensaje = "# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n#\n";
    Ok(format!("{}#\n# Output de status\n#\n", mensaje))
}

/// Lee por stdin y guarda el mensaje introducido.
fn read_from_stdin(stdin: &mut dyn Read) -> Result<String, CommandError> {
    let mut message = String::new();
    let end = "q".to_string();
    loop {
        let mut buf = [0; 1];
        if stdin.read_exact(&mut buf).is_err() {
            return Err(CommandError::StdinError);
        };
        let input = String::from_utf8_lossy(&buf).to_string();
        if check_end_message(&input, &end) {
            break;
        }
        message += &input;
    }
    Ok(message)
}

/// Comprueba si el cliente ha terminado de introducir el mensaje.
fn check_end_message(message: &str, end: &str) -> bool {
    let split_message: Vec<String> = message.lines().map(String::from).collect();
    if let Some(last) = split_message.to_owned().last() {
        if *last == *end {
            return true;
        }
    }
    false
}

/// Devuelve un String sin las líneas que empiezan con '#'.
fn ignore_commented_lines(message: String) -> String {
    let split_message: Vec<&str> = message
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    split_message.join("\n")
}

pub fn read_message_completely(i: usize, args: &[String]) -> Result<(String, usize), CommandError> {
    let mut message = String::new();
    let mut number_of_words: usize = 1;
    message += &args[i + 1];
    let end: char;
    if message.starts_with('"') {
        end = '"';
    } else if message.starts_with('\'') {
        end = '\'';
    } else {
        return Ok((message, number_of_words));
    }

    message = message[1..].to_string();
    for pos in i + 2..args.len() {
        number_of_words += 1;
        message += &format!(" {}", &args[pos]);
        if args[pos].ends_with(end) {
            message = message[..message.len() - 1].to_string();
            break;
        }
        if pos == args.len() - 1 {
            return Err(CommandError::MessageIncomplete(end.to_string()));
        }
    }

    Ok((message, number_of_words))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_invalid_arg() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-no".to_string()];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_invalid_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba2";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-m".to_string()];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::MessageNoValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_empty_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "q\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = [];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::CommitMessageEmptyValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_message_and_reuse() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = [
            "-m".to_string(),
            "message".to_string(),
            "-C".to_string(),
            "hash todavía no se chequea".to_string(),
        ];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::MessageAndReuseError),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_reuse_no_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-m".to_string(), "message".to_string(), "-C".to_string()];
        match Commit::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::ReuseMessageNoValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_enter_message() {
        let input = "#Evitar\nMessage\nq\n";
        let expected = "Message".to_string();
        let mut stdin_mock = Cursor::new(input.as_bytes());

        match run_enter_message(&mut stdin_mock, get_enter_message_text().unwrap()) {
            Err(error) => assert!(false, "{}", error),
            Ok(message) => assert_eq!(message, expected),
        }
    }

    /*
    Aclaración: El resto de las funciones son testeadas en tests/ porque necesitan de la existencia
    de un repositorio y de acciones previas como 'add'.
     */
}
