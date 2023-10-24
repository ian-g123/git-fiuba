use std::{
    fs,
    io::{Read, Write},
    net::ToSocketAddrs,
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        config::Config,
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
        logger.log("Initianlizing Cline");

        let instance = Self::new(args)?;
        logger.log(&format!("Cloning into '{}'...", instance.get_address()));
        logger.log(&format!("repository_path: {}", instance.repository_path));
        logger.log(&format!("repository_url: {}", instance.repository_url));
        logger.log(&format!("repository_port: {:?}", instance.repository_port));
        logger.log(&format!("directory: {}", instance.directory));

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
        let mut url_and_repo = args[i].clone();
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

    fn run(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let directory = self.directory.clone();
        fs::create_dir_all(&directory)
            .map_err(|error| CommandError::DirectoryCreationError(error.to_string()))?;
        let args = &[directory];
        let mut init_output = Vec::new();
        Init::run_from("init", args, stdin, &mut init_output, logger)?;
        let address = self.get_address();
        let url = address + &self.repository_path;
        logger.log("Adding remote");
        let mut config = Config::open()?;
        logger.log("changing remote");
        config.insert("remote \"origin\"", "url", &url);
        logger.log("Saving remote");
        config.save()?;
        fetch(logger)?;
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

fn fetch(logger: &mut Logger) -> Result<(), CommandError> {
    logger.log("Fetching...");
    let config = &Config::open()?;
    // display config contents
    for (domain, configs) in &config.entries {
        logger.log(&format!("[{}]", domain));
        for (key, value) in configs {
            logger.log(&format!("\t{} = {}", key, value));
        }
    }
    let Some(url) = config.get("remote \"origin\"", "url") else {
        return Err(CommandError::NoRemoteUrl);
    };

    let Some((address, repository_path)) = url.split_once('/') else {
        return Err(CommandError::InvalidConfigFile);
    };

    let (repository_url, _repository_port) = {
        match address.split_once(':') {
            Some((repository_url, port)) => (repository_url, Some(port)),
            None => (address, None),
        }
    };

    let mut server = GitServer::connect_to(&address)?;
    let (_head_branch, branch_list) =
        server.explore_repository(repository_path, repository_url, logger)?;
    Ok(for (hash, path) in branch_list {
        logger.log(&format!("{} {}", hash, path));
    })
}
