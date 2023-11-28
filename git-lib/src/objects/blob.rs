use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    option,
    path::Path,
};

use crate::{
    changes_controller_components::changes_types::ChangeType,
    command_errors::CommandError,
    git_repository::get_current_file_content,
    objects_database::ObjectsDatabase,
    utils::{
        aux::get_name,
        super_string::{u8_vec_to_hex_string, SuperStrings},
    },
};
use crate::{logger::Logger, utils::aux::*};

use super::{
    author::Author,
    git_object::{write_to_stream_from_content, GitObject, GitObjectTrait},
    mode::Mode,
    tree::Tree,
};

#[derive(Clone, Debug)]
pub struct Blob {
    content: Option<Vec<u8>>,
    _mode: Mode,
    path: Option<String>,
    hash: Option<[u8; 20]>,
    name: Option<String>,
}

impl Blob {
    /// Crea un Blob a partir de su ruta. Si la ruta no existe, devuelve Error.
    pub fn new_from_path(path: String) -> Result<Self, CommandError> {
        let mode = Mode::get_mode(path.clone())?;
        Ok(Self {
            content: None,
            _mode: mode,
            path: Some(path.clone()),
            hash: None,
            name: Some(get_name(&path)?),
        })
    }

    pub fn new_from_hash_and_name(
        hash: String,
        name: String,
        mode: Mode,
    ) -> Result<Self, CommandError> {
        let hash = hash.cast_hex_to_u8_vec()?;
        Ok(Self {
            content: None,
            _mode: mode,
            path: None,
            hash: Some(hash),
            name: Some(name),
        })
    }

    pub fn new_from_hash_path_and_mode(
        hash: String,
        path: String,
        mode: Mode,
    ) -> Result<Self, CommandError> {
        let hash_vec = hash.cast_hex_to_u8_vec()?;
        let mut hash = [0; 20];
        hash.copy_from_slice(&hash_vec);
        Ok(Self {
            content: None,
            _mode: mode,
            path: Some(path.clone()),
            hash: Some(hash),
            name: Some(get_name(&path)?),
        })
    }

    pub fn new_from_hash_content_and_mode(
        hash: String,
        content: Vec<u8>,
        mode: Mode,
    ) -> Result<Self, CommandError> {
        let hash_vec = hash.cast_hex_to_u8_vec()?;
        let mut hash = [0; 20];
        hash.copy_from_slice(&hash_vec);
        Ok(Self {
            content: Some(content),
            _mode: mode,
            path: None,
            hash: Some(hash),
            name: None,
        })
    }

    /// Crea un Blob a partir de su hash. Si la ruta no existe, devuelve Error.
    pub fn new_from_hash_and_path(hash: [u8; 20], path: &str) -> Result<Self, CommandError> {
        let mode = if path != "" {
            Mode::get_mode(path.to_string())?
        } else {
            Mode::RegularFile
        };
        Ok(Self {
            content: None,
            _mode: mode,
            path: Some(path.to_string()),
            hash: Some(hash),
            name: Some(get_name(&path.to_string())?),
        })
    }

    pub fn new_from_content_and_path(content: Vec<u8>, path: &str) -> Result<Self, CommandError> {
        let mut data: Vec<u8> = Vec::new();
        write_to_stream_from_content(&mut data, content.clone(), "blob".to_string())?;
        let hash = get_sha1(&data);
        let mut instance = Self::new_from_hash_and_path(hash, path)?;
        instance.content = Some(content);
        Ok(instance)
    }

    pub fn new_from_content(content: Vec<u8>) -> Result<Blob, CommandError> {
        let mut data: Vec<u8> = Vec::new();
        write_to_stream_from_content(&mut data, content.clone(), "blob".to_string())?;
        let hash = get_sha1(&data);
        let hash_str = u8_vec_to_hex_string(&hash);
        let instance = Self::new_from_hash_content_and_mode(hash_str, content, Mode::RegularFile)?;
        Ok(instance)
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        _: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut content = vec![0; len as usize];

        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        logger.log(&format!(
            "content: {}",
            String::from_utf8(content.clone()).unwrap()
        ));

        let blob = Blob::new_from_content_and_path(content, path)?;
        logger.log("blob created");
        Ok(Box::new(blob))
    }

    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        len: usize,
        output: &mut dyn Write,
        _logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut content = vec![0; len as usize];
        stream
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let output_str = String::from_utf8(content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        writeln!(output, "{}", output_str)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

impl GitObjectTrait for Blob {
    fn as_mut_blob(&mut self) -> Option<&mut Blob> {
        Some(self)
    }
    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        None
    }
    fn get_path(&self) -> Option<String> {
        self.path.clone()
    }
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "blob".to_string()
    }

    fn content(&mut self, db: Option<&mut ObjectsDatabase>) -> Result<Vec<u8>, CommandError> {
        if let Some(content) = &self.content {
            return Ok(content.to_owned());
        }
        if let Some(db) = db {
            if let Some(hash) = self.hash {
                let mut object =
                    db.read_object(&u8_vec_to_hex_string(&hash), &mut Logger::new_dummy())?;
                let content = object.content(None)?;
                self.content = Some(content.clone());
                return Ok(content);
            }
        }
        match &self.path {
            Some(path) => {
                let content = read_file_contents(path)?;
                Ok(content)
            }
            None => Err(CommandError::FileReadError(
                "Archivo blob inexistente".to_string(),
            )),
        }
    }

    // TODO: implementar otros modos para blobs
    fn mode(&self) -> Mode {
        Mode::RegularFile
    }

    fn to_string_priv(&mut self) -> String {
        //map content to utf8
        let Ok(content) = self.content(None) else {
            return "Error convirtiendo a utf8".to_string();
        };
        let Ok(string) = String::from_utf8(content.clone()) else {
            return "Error convirtiendo a utf8".to_string();
        };
        string
    }

    /// Devuelve el hash del Blob.
    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        if let Some(hash) = self.hash {
            return Ok(hash);
        }
        let mut buf: Vec<u8> = Vec::new();
        self.write_to(&mut buf, None)?;
        let hash = get_sha1(&buf);
        self.set_hash(hash);
        Ok(hash)
    }

    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    fn restore(
        &mut self,
        path: &str,
        logger: &mut Logger,
        db: Option<ObjectsDatabase>,
    ) -> Result<(), CommandError> {
        let mut file = File::create(path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error al crear archivo {}: {}",
                path,
                error.to_string()
            ))
        })?;
        let content = match db {
            Some(mut db) => self.content(Some(&mut db))?,
            None => self.content(None)?,
        };
        logger.log(&format!(
            "Writing in {} the following content:\n{}",
            path,
            String::from_utf8(content.clone()).unwrap()
        ));
        file.write_all(&content).map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error al escribir archivo {}: {}",
                path,
                error.to_string()
            ))
        })?;
        Ok(())
    }

    fn checkout_restore(
        &mut self,
        path: &str,
        logger: &mut Logger,
        deletions: &mut Vec<String>,
        modifications: &mut Vec<String>,
        conflicts: &mut Vec<String>,
        common: &mut Tree,
        unstaged_files: &Vec<String>,
        staged: &HashMap<String, Vec<u8>>,
    ) -> Result<bool, CommandError> {
        if !Path::new(path).exists() && unstaged_files.contains(&path.to_string()) {
            logger.log(&format!("This file was deleted: {}", path));
            deletions.push(path.to_string());
            return Ok(true);
        }
        let mut new_content = self.content(None)?;

        if Path::new(path).exists() {
            let content = get_current_file_content(path)?;

            if new_content != content
                && (unstaged_files.contains(&path.to_string()) || staged.contains_key(path))
            {
                logger.log(&format!("Unstaged: {:?}", unstaged_files));

                logger.log(&format!("This file was modified: {}", path));

                modifications.push(path.to_string());

                new_content = content;
            }
        }
        let mut file = File::create(path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error al crear archivo {}: {}",
                path,
                error.to_string()
            ))
        })?;

        file.write_all(&new_content).map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error al escribir archivo {}: {}",
                path,
                error.to_string()
            ))
        })?;

        if let Some(staged_content) = staged.get(path) {
            self.content = Some(staged_content.to_vec());
            self.hash = Some(get_sha1(&staged_content));
        }

        Ok(false)
    }
}

fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(content)
}
