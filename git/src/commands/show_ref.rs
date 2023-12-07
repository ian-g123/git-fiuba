use std::{io::Read, io::Write};

use crate::commands::command::{Command, ConfigAdderFunction};
use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::check_errors_flags;

/// Hace referencia a un Comando ShowRef.
pub struct ShowRef {
    heads: bool,
    head: bool,
    tags: bool,
    dereference: bool,
    hash: (bool, Option<usize>),
    refs: Vec<String>,
}

impl Command for ShowRef {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "show-ref" {
            return Err(CommandError::Name);
        }

        let mut instance = ShowRef::new_from(args)?;

        instance.run(stdin, output)?;

        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<ShowRef> {
        vec![
            Self::add_heads_config,
            Self::add_head_config,
            Self::add_tags_config,
            Self::add_dereference_config,
            Self::add_hash_config,
            Self::add_refs_config,
        ]
    }
}

impl ShowRef {
    /// Crea un nuevo Comando ShowRef a partir de sus argumentos. Lo configura.
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut instance = Self::new_default();
        instance.config(args)?;
        Ok(instance)
    }

    /// Crea un nuevo Comando ShowRef a partir de valores por default.
    fn new_default() -> Self {
        ShowRef {
            heads: false,
            head: false,
            tags: false,
            dereference: false,
            hash: (false, None),
            refs: Vec::new(),
        }
    }

    /// Configura el flag --heads.
    fn add_heads_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--heads".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.heads = true;

        Ok(i + 1)
    }

    /// Configura el flag --head.
    fn add_head_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--head".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.head = true;

        Ok(i + 1)
    }

    /// Configura el flag --tags.
    fn add_tags_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--tags".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.tags = true;

        Ok(i + 1)
    }

    /// Configura el flag --dereference.
    fn add_dereference_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-d".to_string(), "--dereference".to_string()].to_vec();
        check_errors_flags(i, args, &options)?;
        self.dereference = true;

        Ok(i + 1)
    }

    /// Configura el flag --hash.
    fn add_hash_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let flag = args[i].clone();
        if flag != *"-s" && !flag.starts_with("--hash") {
            return Err(CommandError::WrongFlag);
        }

        if flag.strip_prefix("--hash=").is_some() {
            if flag.len() == 7 {
                return Err(CommandError::FlagHashRequiresValue);
            } else {
                let num_str = flag[7..].to_string();
                let mut digits: i32 = num_str.parse().map_err(|_| CommandError::CastingError)?;
                if digits > 40 || digits == 0 {
                    digits = 40;
                } else if digits < 0 || (digits > 0 && digits < 4) {
                    digits = 4;
                }
                let digits: usize = digits as usize;
                self.hash = (true, Some(digits));
            }
        } else {
            self.hash = (true, None);
        }

        Ok(i + 1)
    }

    /// Configura el flag <ref>>.
    fn add_refs_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        self.refs.push(args[i].clone());

        Ok(i + 1)
    }

    /// Ejecuta el comando Show-ref.
    fn run(&mut self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        let mut show_heads = true;
        let mut show_tags = true;
        let mut show_remotes = true;
        if self.heads && !self.tags {
            show_tags = false;
            show_remotes = false;
        } else if !self.heads && self.tags {
            show_heads = false;
            show_remotes = false;
        } else if self.heads && self.tags {
            show_remotes = false;
        }

        repo.show_ref(
            self.head,
            show_heads,
            show_remotes,
            show_tags,
            self.dereference,
            self.hash,
            &self.refs,
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
        match ShowRef::run_from("commit", &args, &mut stdin_mock, &mut stdout_mock) {
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
        match ShowRef::run_from("show-ref", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_hash_value() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = ["--hash=".to_string()];
        match ShowRef::run_from("show-ref", &args, &mut stdin_mock, &mut stdout_mock) {
            Err(error) => assert_eq!(error, CommandError::FlagHashRequiresValue),
            Ok(_) => panic!(),
        }
    }
}
