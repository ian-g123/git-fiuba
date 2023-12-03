use std::io::{Read, Write};

use crate::commands::command::Command;
use git_lib::{
    command_errors::CommandError,
    git_repository::{self},
    objects::commit_object::{print_for_log, CommitObject},
};

use super::command::ConfigAdderFunction;

/// Commando log
pub struct Log {
    all: bool,
}

impl Command for Log {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "log" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;
        instance.run(output)
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![Self::add_all_config]
    }
}

impl Log {
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut log = Self::new_default();
        log.config(args)?;
        Ok(log)
    }

    fn new_default() -> Self {
        Self { all: false }
    }

    fn add_all_config(log: &mut Log, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--all" {
            return Err(CommandError::WrongFlag);
        }
        log.all = true;
        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = git_repository::GitRepository::open("", output)?;
        let mut commits = repo.get_log(self.all)?;
        print_for_log(output, &mut commits)?;
        Ok(())
    }
}
