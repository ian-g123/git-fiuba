use std::{
    collections::HashMap,
    io::{Read, Write},
};

use super::{command_errors::CommandError, utils::aux::join_paths_m};

pub struct Config {
    entries: HashMap<String, HashMap<String, String>>, // domain, branches, ubicación
    config_path: String,
}

impl Config {
    pub fn open(path: &str) -> Result<Self, CommandError> {
        let config_path_str = join_paths_m(path, ".git/config")?;

        let mut config = Self::default_config(&config_path_str);
        let Ok(mut file) = std::fs::File::open(&config_path_str) else {
            println!(
                "No existe el archivo de configuración en {}",
                &config_path_str
            );
            return Ok(config);
        };
        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            return Err(CommandError::InvalidConfigFile);
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
        let mut file = std::fs::File::create(&self.config_path).map_err(|error| {
            CommandError::FileWriteError("Error guardando config:".to_string() + &error.to_string())
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

    fn default_config(config_path: &str) -> Config {
        let mut entries: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut user_configs: HashMap<String, String> = HashMap::new();
        user_configs.insert("email".to_string(), "example@email.org".to_string());
        user_configs.insert("name".to_string(), "Foo Bar".to_string());
        entries.insert("user".to_string(), user_configs);
        Self {
            entries,
            config_path: config_path.to_string(),
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
