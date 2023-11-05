use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};
/// Commando Clone
pub struct Clone {
    repository_path: String,
    repository_url: String,
    repository_port: Option<String>,
    directory: String,
}

impl Command for Clone {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "clone" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Clone> {
        vec![Clone::add_repository_config, Clone::add_directory_config]
    }
}

impl Clone {
    fn new(args: &[String]) -> Result<Clone, CommandError> {
        let mut clone = Clone::new_default();
        clone.config(args)?;
        clone.directory = clone.get_directory();
        Ok(clone)
    }

    fn new_default() -> Clone {
        Clone {
            repository_path: String::new(),
            repository_url: String::new(),
            repository_port: None,
            directory: String::new(),
        }
    }
    fn add_repository_config(
        clone: &mut Clone,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let url_and_repo = args[i].clone();
        if !url_and_repo.starts_with("git://") {
            return Err(CommandError::InvalidArgument(
                "repository url must start with git://".to_string(),
            ));
        }
        let url_and_repo = &url_and_repo[6..];
        let Some((url_and_port, path)) = url_and_repo.split_once('/') else {
            return Err(CommandError::InvalidArguments);
        };
        clone.repository_path = "/".to_string() + path;
        let Some((url, port)) = url_and_port.split_once(':') else {
            clone.repository_url = url_and_port.to_string();
            return Ok(i + 1);
        };
        clone.repository_url = url.to_string();
        clone.repository_port = Some(port.to_string());
        Ok(i + 1)
    }

    fn add_directory_config(
        clone: &mut Clone,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        clone.directory = args[i].clone();
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::init(&self.directory, "master", false, output)?;
        let address = self.get_address();
        let url = address + &self.repository_path;
        repo.update_remote(url)?;
        repo.pull()?;
        Ok(())
    }

    fn get_address(&self) -> String {
        let Some(repository_port) = self.repository_port.clone() else {
            return self.repository_url.clone();
        };
        return self.repository_url.clone() + ":" + &repository_port;
    }

    fn get_directory(&self) -> String {
        if self.directory.is_empty() {
            return self.repository_path[1..].to_string();
        }
        return self.directory.clone();
    }
}
