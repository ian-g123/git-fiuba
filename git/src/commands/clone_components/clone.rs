use std::io::{Read, Write};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        init_components::init::Init,
        objects::{blob::Blob, git_object::GitObject},
        objects_database,
        server_components::git_server::GitServer,
    },
    logger::Logger,
};

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
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "clone" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        instance.run(stdin, output, logger)?;
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
        let mut url_and_repo = args[i].clone();
        if !url_and_repo.starts_with("git://") {
            return Err(CommandError::InvalidArgument(
                "repository url must start with git://".to_string(),
            ));
        }
        url_and_repo.drain(6..);
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

    fn run(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        Init::run_from("init", &[self.directory.clone()], stdin, output, logger)?;
        let address = self.get_address();
        let mut server = GitServer::connect_to(&address)?;
        let mut lines =
            server.send("git-upload-pack /server-repo\0host=127.1.0.1\0\0version=1\0")?;
        for line in lines {
            logger.log(&line);
        }
        Ok(())
    }

    fn fetch_remote_branches(&self) {}

    fn get_address(&self) -> String {
        let Some(repository_port) = self.repository_port.clone() else {
            return self.repository_url.clone();
        };
        return self.repository_url.clone() + ":" + &repository_port;
    }
}
