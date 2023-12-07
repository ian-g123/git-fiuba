extern crate sha1;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Cursor, Read, Write},
};

use crate::{
    join_paths,
    logger::Logger,
    objects::git_object::{get_type_and_len, git_object_from_data},
    server_components::{
        packfile_functions::search_object_data_from_hash, packfile_object_type::PackfileObjectType,
    },
    utils::aux::{get_sha1_str, hex_string_to_u8_vec, join_paths_m},
};

use super::{
    command_errors::CommandError,
    file_compressor::{compress, extract},
    objects::git_object::GitObject,
};

#[derive(Clone)]
pub struct ObjectsDatabase {
    db_path: String,
}

impl ObjectsDatabase {
    pub fn write(
        &self,
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
        logger.log(&format!("Database: Reading deep object ⚠️: {}", hash_str));
        let (type_str, len, content) = self.read_object_data(hash_str, logger)?;
        return git_object_from_data(
            type_str,
            &mut content.as_slice(),
            len,
            "",
            hash_str,
            logger,
            Some(self),
        );
    }

    /// Dado un hash que representa la ruta del objeto a `.git/objects`, devuelve el objeto que este representa.
    pub fn read_object_shallow(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        logger.log(&format!("Database: Reading shallow object: {}", hash_str));
        let (type_str, len, content) = self.read_object_data(hash_str, logger)?;
        return git_object_from_data(
            type_str,
            &mut content.as_slice(),
            len,
            "",
            hash_str,
            logger,
            None,
        );
    }

    /// Dado un hash que representa la ruta del objeto a `.git/objects`, devuelve su tipo, su longitud y el contenido.
    pub fn read_object_data(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<(String, usize, Vec<u8>), CommandError> {
        logger.log(&format!("Database: Reading data: {}", hash_str));
        //throws error if hash_str is not a valid sha1
        if hash_str.len() != 40 {
            return Err(CommandError::FileOpenError(format!(
                "No es un hash válido {}",
                hash_str
            )));
        }
        let file_path = join_paths!(&self.db_path, &hash_str[0..2], &hash_str[2..])
            .ok_or(CommandError::JoiningPaths)?;
        if !std::path::Path::new(
            &join_paths!(&self.db_path, &hash_str[0..2], &hash_str[2..])
                .ok_or(CommandError::JoiningPaths)?,
        )
        .exists()
        {
            logger.log("Reading data form packfiles");
            let (pack_type, len, content) = self.read_object_data_from_packs(hash_str, logger)?;
            return Ok((pack_type.to_string(), len, content));
        }
        logger.log(&format!("Reading data form path: {}", file_path));

        let mut file = File::open(&file_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error al abrir archivo {}: {}",
                file_path,
                error
            ))
        })?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error al leer archivo {}: {}",
                file_path,
                error
            ))
        })?;
        let decompressed_data = extract(&data)?;
        let mut cursor = Cursor::new(decompressed_data);
        let (type_str, len) = get_type_and_len(&mut cursor)?;
        let mut data = Vec::new();
        cursor.read_to_end(&mut data).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error al leer archivo {}: {}",
                file_path,
                error
            ))
        })?;
        Ok((type_str, len, data))
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
        // si existe el path, no se guarda
        if std::path::Path::new(&path).exists() {
            return Ok(hash_str);
        }
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
        if File::open(file_path).is_err() {
            return Ok(false);
        }
        Ok(true)
    }

    fn read_object_data_from_packs(
        &self,
        hash_str: &str,
        logger: &mut Logger,
    ) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
        let packs = self.get_pack_paths()?;
        for (index_path, packfile_path) in packs {
            let mut index_file = File::open(&index_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al abrir archivo {}: {}",
                    index_path,
                    error
                ))
            })?;
            let mut packfile = File::open(&packfile_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al abrir archivo {}: {}",
                    packfile_path,
                    error
                ))
            })?;
            logger.log(&format!("Searching object in packfile: {}", packfile_path));
            if let Some(object) = search_object_data_from_hash(
                hex_string_to_u8_vec(hash_str),
                &mut index_file,
                &mut packfile,
                self,
                logger,
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
                error
            ))
        })?;
        for pack_file in pack_files {
            let pack_file = pack_file.map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error al leer archivo en directorio {}: {}",
                    packs_path,
                    error
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
