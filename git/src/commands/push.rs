use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Clone
pub struct Push {
    repository_path: String,
    // repository_url: String,
    // repository_port: Option<String>,
    directory: String,
}

impl Command for Push {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "push" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Push> {
        vec![Push::add_repository_config, Push::add_directory_config]
    }
}

impl Push {
    fn new(args: &[String]) -> Result<Push, CommandError> {
        let mut clone = Push::new_default();
        clone.config(args)?;
        // clone.directory = clone.get_directory();
        Ok(clone)
    }

    fn new_default() -> Push {
        Push {
            repository_path: String::new(),
            // repository_url: String::new(),
            // repository_port: None,
            directory: String::new(),
        }
    }

    fn add_repository_config(
        clone: &mut Push,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        Ok(i + 1)
    }

    fn add_directory_config(
        clone: &mut Push,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        //clone.directory = args[i].clone();
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::init(&self.directory, "master", false, output)?;

        repo.push()?;
        Ok(())
    }

    fn get_address(&self) -> String {
        return String::new();
    }

    fn get_directory(&self) -> String {
        return String::new();
    }
}
