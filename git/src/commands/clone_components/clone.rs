use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::ToSocketAddrs,
};

use chrono::format::format;

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
        fetch(&self.directory, logger)?;
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

fn fetch(base_path: &str, logger: &mut Logger) -> Result<(), CommandError> {
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
    update_remote_branches(
        &mut server,
        repository_path,
        repository_url,
        logger,
        base_path,
    )?;
    fetch_and_save_objects(&mut server, logger, base_path)?;
    Ok(())
}

fn fetch_and_save_objects(
    server: &mut GitServer,
    logger: &mut Logger,
    base_path: &str,
) -> Result<(), CommandError> {
    let wants = remote_branches(base_path)?.into_values().collect();
    let haves = local_branches(base_path)?.into_values().collect();
    let objects_decompressed_data = server.fetch_objects(wants, haves, logger)?;
    Ok(())
}

fn local_branches(base_path: &str) -> Result<HashMap<String, String>, CommandError> {
    let mut branches = HashMap::<String, String>::new();
    let branches_path = format!("{}/.git/refs/heads/", base_path);
    let paths = fs::read_dir(branches_path).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error leyendo directorio de branches: {}",
            error.to_string()
        ))
    })?;
    for path in paths {
        let path = path.map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let file_name = &path.file_name();
        let Some(file_name) = file_name.to_str() else {
            return Err(CommandError::FileReadError(
                "Error leyendo directorio de branches".to_string(),
            ));
        };
        let mut file = fs::File::open(path.path()).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let mut sha1 = String::new();
        file.read_to_string(&mut sha1).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        branches.insert(file_name.to_string(), sha1);
    }
    Ok(branches)
}

fn remote_branches(base_path: &str) -> Result<HashMap<String, String>, CommandError> {
    let mut branches = HashMap::<String, String>::new();
    let branches_path = format!("{}/.git/refs/remotes/origin/", base_path);
    let paths = fs::read_dir(branches_path).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error leyendo directorio de branches: {}",
            error.to_string()
        ))
    })?;
    for path in paths {
        let path = path.map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let file_name = &path.file_name();
        let Some(file_name) = file_name.to_str() else {
            return Err(CommandError::FileReadError(
                "Error leyendo directorio de branches".to_string(),
            ));
        };
        let mut file = fs::File::open(path.path()).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let mut sha1 = String::new();
        file.read_to_string(&mut sha1).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        branches.insert(file_name.to_string(), sha1);
    }
    Ok(branches)
}

fn update_remote_branches(
    server: &mut GitServer,
    repository_path: &str,
    repository_url: &str,
    logger: &mut Logger,
    base_path: &str,
) -> Result<(), CommandError> {
    let (_head_branch, branch_remote_refs) =
        server.explore_repository(&("/".to_owned() + repository_path), repository_url, logger)?;
    Ok(for (sha1, mut ref_path) in branch_remote_refs {
        ref_path.replace_range(0..11, "");
        update_ref(&base_path, &sha1, &ref_path, logger)?;
    })
}

fn update_ref(
    base_path: &str,
    sha1: &str,
    ref_name: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let dir_path = format!("{}/.git/refs/remotes/origin/", base_path);
    let file_path = dir_path.to_owned() + ref_name;

    fs::create_dir_all(dir_path).unwrap();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&file_path)
        .map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error guardando ref en {}: {}",
                file_path,
                &error.to_string()
            ))
        })?;
    file.write_all(sha1.as_bytes()).map_err(|error| {
        CommandError::FileWriteError("Error guardando ref:".to_string() + &error.to_string())
    })?;
    Ok(())
}
