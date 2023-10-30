use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Fetch
pub struct Fetch {
    repository: Option<String>,
}

impl Command for Fetch {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "fetch" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Fetch> {
        vec![Fetch::add_repository_config]
    }
}

impl Fetch {
    fn new(args: &[String]) -> Result<Fetch, CommandError> {
        println!("fetch args: {:?}", args);
        let mut fetch = Fetch::new_default();
        fetch.config(args)?;

        Ok(fetch)
    }

    fn new_default() -> Fetch {
        Fetch { repository: None }
    }
    fn add_repository_config(
        fetch: &mut Fetch,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        fetch.repository = Some(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.fetch()?;
        Ok(())
    }
}
