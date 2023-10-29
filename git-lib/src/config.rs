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
// Example file:
// [domain]
// 	key = value
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
                    println!(
                        "Inserting [{}]{} = {}",
                        current_domain,
                        key.trim(),
                        value.trim()
                    );
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

fn fetch_and_save_objects(server: &mut GitServer, base_path: &str) -> Result<(), CommandError> {
    let wants = remote_branches(base_path)?.into_values().collect();
    let haves = local_branches(base_path)?.into_values().collect();
    let objects_decompressed_data = server.fetch_objects(wants, haves)?;
    for (obj_type, len, content) in objects_decompressed_data {
        // logger.log(&format!(
        //     "Saving object of type {} and len {}, with data {:?}",
        //     obj_type,
        //     len,
        //     String::from_utf8_lossy(&content)
        // ));
        let mut git_object: GitObject =
            Box::new(ProtoObject::new(content, len, obj_type.to_string()));
        objects_database::write_to(
            &mut Logger::new_dummy(),
            &mut git_object,
            &format!("{}/", base_path),
        )?;
    }
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
    base_path: &str,
) -> Result<(), CommandError> {
    let (_head_branch, branch_remote_refs) =
        server.explore_repository(&("/".to_owned() + repository_path), repository_url)?;
    Ok(for (sha1, mut ref_path) in branch_remote_refs {
        ref_path.replace_range(0..11, "");
        update_ref(&base_path, &sha1, &ref_path)?;
    })
}

fn update_ref(base_path: &str, sha1: &str, ref_name: &str) -> Result<(), CommandError> {
    let dir_path = format!("{}/.git/refs/remotes/origin/", base_path);
    let file_path = dir_path.to_owned() + ref_name;

    fs::create_dir_all(dir_path).map_err(|error| {
        CommandError::DirectoryCreationError(format!(
            "Error creando directorio de refs: {}",
            error.to_string()
        ))
    })?;
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
