extern crate sha1;
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
};

use crate::{
    join_paths,
    logger::Logger,
    utils::aux::{get_sha1_str, join_paths_m},
};

use super::{
    command_errors::CommandError,
    file_compressor::{compress, extract},
    objects::git_object::{read_git_object_from, GitObject},
};

pub struct ObjectsDatabase {
    db_path: String,
}

impl ObjectsDatabase {
    pub fn write(
        &mut self,
        git_object: &mut GitObject,
        recursive: bool,
        _logger: &mut Logger,
    ) -> Result<String, CommandError> {
        let mut data = Vec::new();
        if recursive {
            git_object.write_to(&mut data, Some(self))?;
        } else {
            git_object.write_to(&mut data, None)?;
        };
        let hash_str = get_sha1_str(&data);
        self.save_to(hash_str, data)
    }

    /// Dado un hash que representa la ruta del objeto a `.git/objects`, devuelve el objeto que este representa.
    pub fn read_object(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        logger.log(&format!("read_object hash_str: {}", hash_str));
        let (path, decompressed_data) = self.read_file(hash_str, logger)?;
        logger.log(&format!("read_object Success reading file!"));
        logger.log(&format!(
            "decompressed_data: {:?}",
            String::from_utf8_lossy(decompressed_data.as_slice())
        ));
        let mut stream = Cursor::new(decompressed_data);
        read_git_object_from(self, &mut stream, &path, &hash_str, logger)
    }

    /// Dado un hash que representa la ruta del objeto a `.git/objects`, devuelve la ruta del objeto y su data descomprimida.
    pub fn read_file(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<(String, Vec<u8>), CommandError> {
        //throws error if hash_str is not a valid sha1
        if hash_str.len() != 40 {
            return Err(CommandError::FileOpenError(format!(
                "No es un hash válido {}",
                hash_str
            )));
        }
        let file_path = join_paths!(&self.db_path, &hash_str[0..2], &hash_str[2..])
            .ok_or(CommandError::JoiningPaths)?;
        logger.log(&format!("read_file file_path: {}", file_path));
        println!("read_file file_path: {}", file_path);
        let mut file = File::open(&file_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error al abrir archivo {}: {}",
                file_path,
                error.to_string()
            ))
        })?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error al leer archivo {}: {}",
                file_path,
                error.to_string()
            ))
        })?;
        let decompressed_data = extract(&data)?;
        Ok((file_path, decompressed_data))
    }

    /// Dado la ruta del repositorio, crea el objeto `ObjectsDatabase` que contiene métodos útiles para
    /// acceder a la base de datos de objetos de git, tanto para leer como para escribir.
    pub(crate) fn new(base_path_str: &str) -> Result<Self, CommandError> {
        Ok(ObjectsDatabase {
            db_path: join_paths_m(base_path_str, ".git/objects")?.to_string(),
        })
    }

    fn save_to(&self, hash_str: String, data: Vec<u8>) -> Result<String, CommandError> {
        let parent_path = join_paths_m(&self.db_path, &hash_str[0..2])?;
        let path = join_paths_m(&parent_path, &hash_str[2..])?;
        if let Err(error) = fs::create_dir_all(parent_path) {
            return Err(CommandError::FileOpenError(error.to_string()));
        };
        let Ok(mut file) = File::create(&path) else {
            return Err(CommandError::FileOpenError(
                "Error al abrir archivo para escritura".to_string(),
            ));
        };
        let compressed_data = compress(&data)?;
        if let Err(error) = file.write_all(&compressed_data) {
            return Err(CommandError::FileWriteError(error.to_string()));
        };
        Ok(hash_str)
    }
}
