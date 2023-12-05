use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Clone
pub struct Push {
    all: bool,
    // remote: String,
    branch: String,
}

impl Command for Push {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "push" {
            return Err(CommandError::Name);
        }
        let push = Push::new(args, output)?;
        push.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Push> {
        vec![Push::add_all_config]
    }
}

impl Push {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Push, CommandError> {
        if args.len() > 1 {
            return Err(CommandError::InvalidArguments);
        }
        let mut push = Push::new_default(output)?;
        if args.len() == 1 {
            push.config(args)?;
        }
        Ok(push)
    }

    pub fn new_default(_output: &mut dyn Write) -> Result<Push, CommandError> {
        Ok(Push {
            all: false,
            // remote: "origin".to_string(),
            branch: "".to_string()
        })
    }

    fn add_all_config(push: &mut Push, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] == "--all" {
            push.all = true;
            Ok(i + 1)
        } else {
            Err(CommandError::InvalidArguments)
        }
    }

    pub fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.push(self.all, self.branch.to_owned())?;
        Ok(())
    }
}
