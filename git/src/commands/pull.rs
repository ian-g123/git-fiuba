use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};
/// Commando Pull
pub struct Pull {}

impl Command for Pull {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "pull" {
            return Err(CommandError::Name);
        }
        let pull = Self::new(args)?;

        pull.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Pull> {
        vec![]
    }
}

impl Pull {
    fn new(args: &[String]) -> Result<Pull, CommandError> {
        if args.len() != 1 {
            return Err(CommandError::PullError(
                "Pull command takes no arguments".to_string(),
            ));
        }
        let pull = Pull::new_default();
        Ok(pull)
    }

    fn new_default() -> Pull {
        Pull {}
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.pull()?;
        Ok(())
    }
}
