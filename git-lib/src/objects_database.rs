extern crate sha1;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Cursor, Read, Write},
};

use chrono::format;

use crate::{
    join_paths,
    logger::Logger,
    server_components::packfile_functions::search_object_from_hash,
    utils::aux::{get_sha1_str, hex_string_to_u8_vec, join_paths_m},
};

use super::{
    command_errors::CommandError,
    file_compressor::{compress, extract},
    objects::git_object::{read_git_object_from, GitObject},
};

#[derive(Clone)]
pub struct ObjectsDatabase {
    db_path: String,
}

impl ObjectsDatabase {
    pub fn write(
        &mut self,
        git_object: &mut GitObject,
        recursive: bool,
        logger: &mut Logger,
    ) -> Result<String, CommandError> {
        let mut data = Vec::new();
        if recursive {
            git_object.write_to(&mut data, Some(self))?;
        } else {
            git_object.write_to(&mut data, None)?;
        };
        let hash_str = get_sha1_str(&data);
        logger.log(&format!("Writing {} to database", hash_str));
        self.save_to(hash_str, data)
    }

    /// Dado un hash que representa la ruta del objeto a `.git/objects`, devuelve el objeto que este representa.
    pub fn read_object(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        logger.log(&format!("read_object hash_str: {}", hash_str));

        if std::path::Path::new(
            &join_paths!(&self.db_path, &hash_str[0..2], &hash_str[2..])
                .ok_or(CommandError::JoiningPaths)?,
        )
        .exists()
        {
            let (path, decompressed_data) = self.read_file(hash_str, logger)?;
            logger.log(&format!("read_object Success reading file!"));
            logger.log(&format!(
                "decompressed_data: {:?}",
                String::from_utf8_lossy(decompressed_data.as_slice())
            ));
            let mut stream = Cursor::new(decompressed_data);
            return read_git_object_from(self, &mut stream, &path, &hash_str, logger);
        }
        self.read_object_from_packs(hash_str, logger)
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
        logger.log(&format!("Database reading: {}", file_path));

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
    pub(crate) fn new(git_path: &str) -> Result<Self, CommandError> {
        Ok(ObjectsDatabase {
            db_path: join_paths_m(git_path, "objects")?.to_string(),
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

    pub fn has_object(&self, hash_str: &str) -> Result<bool, CommandError> {
        //throws error if hash_str is not a valid sha1
        if hash_str.len() != 40 {
            return Ok(false);
        }
        let file_path = join_paths!(&self.db_path, &hash_str[0..2], &hash_str[2..])
            .ok_or(CommandError::JoiningPaths)?;
        if File::open(&file_path).is_err() {
            return Ok(false);
        }
        Ok(true)
    }

    fn read_object_from_packs(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let packs = self.get_pack_paths()?;
        for (index_path, packfile_path) in packs {
            let mut index_file = File::open(&index_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al abrir archivo {}: {}",
                    index_path,
                    error.to_string()
                ))
            })?;
            let mut packfile = File::open(&packfile_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al abrir archivo {}: {}",
                    packfile_path,
                    error.to_string()
                ))
            })?;
            if let Some(object) = search_object_from_hash(
                hex_string_to_u8_vec(hash_str),
                &mut index_file,
                &mut packfile,
                &self,
            )? {
                return Ok(object);
            }
        }
        Err(CommandError::FileOpenError(format!(
            "No se encontró el objeto {}",
            hash_str
        )))
    }

    fn get_pack_paths(&self) -> Result<Vec<(String, String)>, CommandError> {
        let packs_path = join_paths!(&self.db_path, "pack").ok_or(CommandError::JoiningPaths)?;
        let mut indices = HashSet::new();
        let mut packs = HashSet::new();
        let pack_files = fs::read_dir(&packs_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error al abrir directorio {}: {}",
                packs_path,
                error.to_string()
            ))
        })?;
        for pack_file in pack_files {
            let pack_file = pack_file.map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al leer archivo en directorio {}: {}",
                    packs_path,
                    error.to_string()
                ))
            })?;
            let filename = pack_file.file_name().into_string().map_err(|_| {
                CommandError::FileOpenError(format!(
                    "Error al convertir nombre de archivo en directorio {}",
                    packs_path
                ))
            })?;
            if !filename.starts_with("pack-") {
                continue;
            }
            if filename.ends_with(".idx") {
                let pack_name = filename[0..filename.len() - 4].to_string();
                indices.insert(pack_name);
            } else if filename.ends_with(".pack") {
                let pack_name = filename[0..filename.len() - 5].to_string();
                packs.insert(pack_name);
            }
        }
        let mut filenames = Vec::new();
        for pack_name in packs {
            if indices.contains(&pack_name) {
                filenames.push((
                    join_paths!(&packs_path, &format!("{}.idx", pack_name))
                        .ok_or(CommandError::JoiningPaths)?,
                    join_paths!(&packs_path, &format!("{}.pack", pack_name))
                        .ok_or(CommandError::JoiningPaths)?,
                ));
            }
        }
        Ok(filenames)
    }
}
