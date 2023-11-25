use std::{io::Read, io::Write};

use crate::commands::command::{Command, ConfigAdderFunction};
use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::{
    command::check_errors_flags,
    commit::{read_message_completely, run_enter_message},
};

/// Hace referencia a un Comando Tag.
pub struct Tag {
    name: String,
    message: Option<String>,
    object: Option<String>,
    create_tag: bool,
    force: bool,
    delete: Vec<String>,
}

impl Command for Tag {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "tag" {
            return Err(CommandError::Name);
        }

        let mut instance = Tag::new_from(args)?;

        instance.run(stdin, output)?;

        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Tag> {
        vec![
            Self::add_message_config,
            Self::add_name_config,
            Self::add_object_config,
            Self::add_create_config,
            Self::add_delete_config,
            Self::add_force_config,
        ]
    }
}

impl Tag {
    /// Crea un nuevo Comando Tag a partir de sus argumentos. Lo configura.
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut instance = Self::new_default();
        instance.config(args)?;
        Ok(instance)
    }

    /// Crea un nuevo Comando Tag a partir de valores por default.
    fn new_default() -> Self {
        Tag {
            name: "".to_string(),
            message: None,
            object: None,
            create_tag: false,
            force: false,
            delete: Vec::new(),
        }
    }

    /// Configura el flag -m.
    fn add_message_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-m".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::MessageNoValue)?;
        if !self.delete.is_empty() {
            return Err(CommandError::TagCreateAndDelete);
        }
        let mut new_message: String = String::new();
        if let Some(message) = &self.message {
            new_message = format!("{}\n\n", message)
        }
        let (message, words) = read_message_completely(i, args)?;
        new_message += &message;
        self.message = Some(new_message);
        self.create_tag = true;
        Ok(i + words + 1)
    }

    /// Configura el nombre del tag.
    fn add_name_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) || !self.name.is_empty() {
            return Err(CommandError::WrongFlag);
        }
        self.name = args[i].clone();
        Ok(i + 1)
    }

    /// Configura el objeto al que apunta el tag.
    fn add_object_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) || self.name.is_empty() {
            return Err(CommandError::WrongFlag);
        }
        if self.object.is_some() {
            return Err(CommandError::TagTooManyArgs);
        }

        self.object = Some(args[i].clone());
        Ok(i + 1)
    }

    /// Configura el flag -a.
    fn add_create_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-a".to_string(), "--annotate".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        if !self.delete.is_empty() {
            return Err(CommandError::TagCreateAndDelete);
        }
        self.create_tag = true;
        Ok(i + 1)
    }

    /// Configura el flag -f.
    fn add_force_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-f".to_string(), "--force".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.force = true;
        Ok(i + 1)
    }

    /// Configura el flag -d.
    fn add_delete_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-d".to_string(), "--delete".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        if self.create_tag || self.message.is_some() || self.force {
            return Err(CommandError::TagCreateAndDelete);
        }

        self.name = "".to_string();
        self.object = None;
        for arg in 0..args.len() {
            if Self::is_flag(&args[arg]) && !options.contains(&args[arg]) {
                return Err(CommandError::TagCreateAndDelete);
            }
            if !options.contains(&args[arg]) {
                self.delete.push(args[arg].clone());
            }
        }
        Ok(args.len())
    }

    /// Ejecuta el comando Tag.
    fn run(&mut self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let message = self.get_tag_message(stdin)?;
        if self.create_tag && message.is_empty() {
            return Err(CommandError::TagMessageEmpty);
        }

        let mut repo = GitRepository::open("", output)?;

        if self.name.is_empty()
            && self.object.is_none()
            && !self.create_tag
            && !self.force
            && self.delete.is_empty()
        {
            return repo.list_tags();
        }

        if !self.delete.is_empty() {
            repo.delete_tags(&self.delete)?;
        } else {
            if self.name.is_empty() {
                return Err(CommandError::TagNameNeeded);
            }
            repo.create_tag(
                &self.name,
                &message,
                self.object.clone(),
                self.create_tag,
                self.force,
            )?;
        }

        Ok(())
    }

    /// Devuelve el mensage del tag.
    fn get_tag_message(&self, stdin: &mut dyn Read) -> Result<String, CommandError> {
        let message = {
            if let Some(message) = self.message.clone() {
                message
            } else if !self.create_tag {
                "".to_string()
            } else {
                let stdout = self.get_enter_message_text_tag();
                run_enter_message(stdin, stdout)?
            }
        };
        Ok(message)
    }

    /// Devuelve el mensaje que se muestra si se intenta crear un tag (-a) sin pasarle un mensaje.
    fn get_enter_message_text_tag(&self) -> String {
        format!(
            "#\n# Write a message for tag:\n#   {}\n# Lines starting with '#' will be ignored.",
            self.name
        )
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
        match Tag::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
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
        match Tag::run_from("tag", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_too_many_args() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["name".to_string(), "ref".to_string(), "third".to_string()];
        match Tag::run_from("tag", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::TagTooManyArgs),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_create_and_delete() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["-a".to_string(), "name".to_string(), "-d".to_string()];
        match Tag::run_from("tag", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::TagCreateAndDelete),
            Ok(_) => assert!(false),
        }
    }
}
