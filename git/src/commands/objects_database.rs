extern crate sha1;
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
};

use sha1::{Digest, Sha1};

use crate::logger::Logger;

use super::{
    command_errors::CommandError,
    file_compressor::{compress, extract},
    objects::git_object::{read_git_object_from, GitObject},
};

pub(crate) fn write(git_object: GitObject) -> Result<String, CommandError> {
    let mut data = Vec::new();
    git_object.write_to(&mut data)?;

    let hex_string = get_sha1_str(&data);
    let folder_name = &hex_string[0..2];
    let parent_path = format!(".git/objects/{}", folder_name);
    let file_name = &hex_string[2..];
    let path = format!("{}/{}", parent_path, file_name);

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
    return Ok(hex_string);
}

fn get_sha1_str(data: &[u8]) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(data);
    let hash_result = sha1.finalize();

    // Formatea los bytes del hash en una cadena hexadecimal
    let hex_string = hash_result
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("");

    hex_string
}

pub(crate) fn read(path: &str, logger: &mut Logger) -> Result<GitObject, CommandError> {
    let mut file = File::open(path).map_err(|error| {
        CommandError::FileOpenError(format!(
            "Error al abrir archivo {}: {}",
            path,
            error.to_string()
        ))
    })?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error al leer archivo {}: {}",
            path,
            error.to_string()
        ))
    })?;
    let mut hash_str = path.split('/').collect::<Vec<&str>>();
    let Some(hash_str_2) = hash_str.pop() else {
        return Err(CommandError::FileReadError(
            "Error al reconstruir el hash del objeto".to_string(),
        ));
    };
    let Some(hash_str_1) = hash_str.pop() else {
        return Err(CommandError::FileReadError(
            "Error al reconstruir el hash del objeto".to_string(),
        ));
    };
    let hash_str = format!("{}{}", hash_str_1, hash_str_2);
    let decompressed_data = extract(&data)?;
    let mut stream = Cursor::new(decompressed_data);
    logger.log("Leyendo objeto");
    read_git_object_from(&mut stream, path, &hash_str, logger)
}
