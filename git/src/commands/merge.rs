use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Merge
pub struct Merge {
    commits: Vec<String>,
}

impl Command for Merge {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "merge" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Merge> {
        vec![Merge::add_commit_config]
    }
}

impl Merge {
    fn new(args: &[String]) -> Result<Merge, CommandError> {
        let mut merge = Merge::new_default();
        merge.config(args)?;

        Ok(merge)
    }

    fn new_default() -> Merge {
        Merge {
            commits: Vec::new(),
        }
    }

    fn add_commit_config(
        merge: &mut Merge,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        merge.commits.push(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.merge(&self.commits)?;
        Ok(())
    }
}
