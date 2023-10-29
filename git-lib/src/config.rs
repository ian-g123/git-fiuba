use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
};

use crate::{
    logger::Logger,
    objects::{git_object::GitObject, proto_object::ProtoObject},
    objects_database,
    server_components::git_server::GitServer,
};

use super::command_errors::CommandError;

pub struct Config {
    entries: HashMap<String, HashMap<String, String>>,

    path: String,
}

impl Config {
    pub fn open(path: &str) -> Result<Self, CommandError> {
        let mut config = Self::default_config(path);
        let Ok(mut file) = std::fs::File::open(format!("{}/.git/config", path)) else {
            return Ok(config);
        };
        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            return Ok(config);
        }
        let mut lines = content.lines();
        let mut current_domain = String::new();
        while let Some(line) = lines.next() {
            if line.starts_with('[') {
                current_domain = line[1..line.len() - 1].to_string();
            } else {
                if current_domain.is_empty() {
                    return Err(CommandError::InvalidConfigFile);
                }
                if let Some((key, value)) = line.split_once('=') {
                    config.insert(&current_domain, key.trim(), value.trim());
                } else {
                    return Err(CommandError::InvalidConfigFile);
                }
            }
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<(), CommandError> {
        let mut file =
            std::fs::File::create(format!("{}/.git/config", self.path)).map_err(|error| {
                CommandError::FileWriteError(
                    "Error guardando config:".to_string() + &error.to_string(),
                )
            })?;

        for (domain, configs) in &self.entries {
            let line = format!("[{domain}]\n");
            file.write_all(line.as_bytes()).map_err(|error| {
                CommandError::FileWriteError(
                    "Error guardando config:".to_string() + &error.to_string(),
                )
            })?;
            for (key, value) in configs {
                let line = format!("\t{} = {}\n", key, value);
                file.write_all(line.as_bytes()).map_err(|error| {
                    CommandError::FileWriteError(
                        "Error guardando config:".to_string() + &error.to_string(),
                    )
                })?;
            }
        }
        Ok(())
    }

    pub fn get_entries(&self) -> HashMap<String, HashMap<String, String>> {
        self.entries.clone()
    }

    fn default_config(path: &str) -> Config {
        let mut entries: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut user_configs: HashMap<String, String> = HashMap::new();
        user_configs.insert("email".to_string(), "example@email.org".to_string());
        user_configs.insert("name".to_string(), "Foo Bar".to_string());
        entries.insert("user".to_string(), user_configs);
        Self {
            entries,
            path: path.to_string(),
        }
    }

    pub fn get(&self, domain: &str, key: &str) -> Option<&String> {
        self.entries.get(domain)?.get(key)
    }

    pub fn insert(&mut self, domain: &str, key: &str, value: &str) {
        if let Some(configs) = self.entries.get_mut(domain) {
            configs.insert(key.to_string(), value.to_string());
        } else {
            let mut configs = HashMap::new();
            configs.insert(key.to_string(), value.to_string());
            self.entries.insert(domain.to_string(), configs);
        }
    }
}
