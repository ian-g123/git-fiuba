use std::io::{Read, Write};

use git_lib::{
    command_errors::CommandError,
    git_repository::{self, GitRepository},
};

use crate::commands::command::{Command, ConfigAdderFunction};

/// Commando init
pub struct Remote {
    verbose: bool,
    add_remote: Vec<String>,
    change_remote_name: Vec<String>,
    remove_remote: Vec<String>,
    rename_remote: Vec<String>,
}

impl Command for Remote {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "remote" {
            return Err(CommandError::Name);
        }

        let mut remote = Remote::new(args)?;

        remote.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Remote> {
        vec![Remote::add_configs]
    }
}

impl Remote {
    fn new(args: &[String]) -> Result<Remote, CommandError> {
        let mut remote = Remote::new_default();
        remote.config(args)?;
        Ok(remote)
    }

    fn new_default() -> Remote {
        Remote {
            verbose: false,
            add_remote: [].to_vec(),
            change_remote_name: [].to_vec(),
            remove_remote: [].to_vec(),
            rename_remote: [].to_vec(),
        }
    }

    fn add_configs(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let i = i;
        if Remote::verbose_config(self, i, args)? {
            return Ok(i + 1);
        } else if Remote::add_config(self, i, args)? {
            return Ok(i + 1);
        } else if Remote::change_url_config(self, i, args)? {
            return Ok(i + 1);
        } else if Remote::remove_config(self, i, args)? {
            return Ok(i + 1);
        } else if Remote::rename_config(self, i, args)? {
            return Ok(i + 1);
        }
        Err(CommandError::InvalidArguments)
    }

    fn verbose_config(
        remote: &mut Remote,
        i: usize,
        args: &[String],
    ) -> Result<bool, CommandError> {
        if args[i] != "-v" {
            return Ok(false);
        }
        if args[i] != "--verbose" {
            return Ok(false);
        }
        if args.len() != 2 {
            return Err(CommandError::InvalidArguments);
        }
        remote.verbose = true;
        Ok(true)
    }

    fn add_config(remote: &mut Remote, i: usize, args: &[String]) -> Result<bool, CommandError> {
        if args[i] != "add" {
            return Ok(false);
        }
        if args.len() != 3 {
            return Err(CommandError::InvalidArguments);
        }
        let remote_name = args[i + 1].clone();
        let remote_url = args[i + 2].clone();
        remote.add_remote = [remote_name, remote_url].to_vec();
        Ok(true)
    }

    fn change_url_config(
        remote: &mut Remote,
        i: usize,
        args: &[String],
    ) -> Result<bool, CommandError> {
        if args[i] != "set-url" {
            return Ok(false);
        }
        if args.len() != 3 {
            return Err(CommandError::InvalidArguments);
        }
        let remote_name = args[i + 1].clone();
        let new_url = args[i + 2].clone();
        remote.change_remote_name = [remote_name, new_url].to_vec();
        Ok(true)
    }

    fn remove_config(remote: &mut Remote, i: usize, args: &[String]) -> Result<bool, CommandError> {
        if args[i] != "remove" {
            return Ok(false);
        }
        if args.len() != 2 {
            return Err(CommandError::InvalidArguments);
        }
        let remote_name = args[i + 1].clone();
        remote.remove_remote = [remote_name].to_vec();
        Ok(true)
    }

    fn rename_config(remote: &mut Remote, i: usize, args: &[String]) -> Result<bool, CommandError> {
        if args[i] != "rename" {
            return Ok(false);
        }
        if args.len() != 3 {
            return Err(CommandError::InvalidArguments);
        }
        let new_name = args[i + 1].clone();
        let old_name = args[i + 2].clone();
        remote.rename_remote = [new_name, old_name].to_vec();
        Ok(true)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut git_repository = GitRepository::open("", output)?;
        if self.add_remote.len() != 0 {
            git_repository.add_remote(self.add_remote[0].as_str(), self.add_remote[1].as_str())?;
        }
        Ok(())
    }
}
