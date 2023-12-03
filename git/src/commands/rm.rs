use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Add
pub struct Rm {
    pathspecs: Vec<String>,
    recursive: bool,
    force: bool,
}

impl Command for Rm {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "rm" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Rm> {
        vec![Rm::rm_file_config, Rm::rm_dir_config, Rm::rm_force_config]
    }
}

impl Rm {
    fn new(args: &[String]) -> Result<Rm, CommandError> {
        let mut rm = Rm::new_default();
        rm.config(args)?;
        Ok(rm)
    }

    fn new_default() -> Rm {
        Rm {
            pathspecs: Vec::<String>::new(),
            recursive: false,
            force: false,
        }
    }

    fn rm_dir_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "-r" {
            return Err(CommandError::WrongFlag);
        }
        rm.recursive = true;
        Ok(i + 1)
    }

    fn rm_force_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--force" || args[i] != "-f" {
            return Err(CommandError::WrongFlag);
        }
        rm.force = true;
        Ok(i + 1)
    }

    fn rm_file_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        rm.pathspecs.push(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.rm(self.pathspecs.clone(), self.recursive, self.force)?;
        Ok(())
    }
}
