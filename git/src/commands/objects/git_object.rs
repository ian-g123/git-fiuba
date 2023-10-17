use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{blob::Blob, mode::Mode, tree::Tree};
use std::{
    fmt,
    io::{Read, Write},
};

pub type GitObject = Box<dyn GitObjectTrait>;

pub trait GitObjectTrait: fmt::Display {
    fn as_mut_tree(&mut self) -> Option<&mut Tree>;

    fn clone_object(&self) -> GitObject;

    fn write_to(&self, stream: &mut dyn std::io::Write) -> Result<(), CommandError> {
        let type_str = self.type_str();
        let content = self.content()?;
        let len = content.len();
        let header = format!("{} {}\0", type_str, len);
        stream
            .write(header.as_bytes())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        stream
            .write(content.as_slice())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn add_path(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        Err(CommandError::ObjectNotTree)
    }

    fn type_str(&self) -> String;

    fn mode(&self) -> Mode;

    fn content(&self) -> Result<Vec<u8>, CommandError>;

    fn size(&self) -> Result<usize, CommandError> {
        let content = self.to_string_priv();
        Ok(content.len())
    }

    fn to_string_priv(&self) -> String;
}

pub fn display_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (path, content) = objects_database::read_file(hash)?;
    let mut stream = std::io::Cursor::new(content);
    let (type_str, len) = get_type_and_len(&mut stream, logger)?;
    if type_str == "blob" {
        return Blob::display_from_hash(&mut stream, len, path, hash, output, logger);
    }
    if type_str == "tree" {
        return Tree::display_from_hash(&mut stream, len, path, hash, output, logger);
    };
    Err(CommandError::ObjectTypeError)
}

pub fn display_type_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = objects_database::read_file(hash)?;
    let mut stream = std::io::Cursor::new(content);
    let (type_str, _) = get_type_and_len(&mut stream, logger)?;
    writeln!(output, "{}", type_str)
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
    Ok(())
}

pub fn display_size_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = objects_database::read_file(hash)?;
    let mut stream = std::io::Cursor::new(content);
    let (_, len) = get_type_and_len(&mut stream, logger)?;
    writeln!(output, "{}", len).map_err(|error| CommandError::FileWriteError(error.to_string()))?;
    Ok(())
}

pub fn read_git_object_from(
    stream: &mut dyn Read,
    path: &str,
    hash_str: &str,
    logger: &mut Logger,
) -> Result<GitObject, CommandError> {
    logger.log("read_git_object_from");
    // let fns = {
    //     "blob": Blob::read_from,
    //     "tree": Tree::read_from,
    //     // Commit::read_from,
    //     // Tag::read_from,
    // };

    let (type_str, len) = get_type_and_len(stream, logger)?;

    logger.log(&format!("len: {}", len));

    if type_str == "blob" {
        return Blob::read_from(stream, len, path, hash_str, logger);
    };
    if type_str == "tree" {
        return Tree::read_from(stream, len, path, hash_str, logger);
    };

    // for read_from in fns {
    //     match read_from(stream, path, hash_str, logger) {
    //         Ok(git_object) => return Ok(git_object),
    //         Err(CommandError::ObjectTypeError) => continue,
    //         Err(error) => return Err(error),
    //     }
    // }
    Err(CommandError::ObjectTypeError)
}

fn get_type_and_len(
    stream: &mut dyn Read,
    logger: &mut Logger,
) -> Result<(String, usize), CommandError> {
    let mut bytes = stream.bytes();
    let type_str = get_type(&mut bytes)?;
    let len_str = get_len(&mut bytes)?;
    logger.log("found \0");
    let len: usize = len_str
        .parse()
        .map_err(|_| CommandError::ObjectLengthParsingError)?;
    Ok((type_str, len))
}

fn get_type(bytes: &mut std::io::Bytes<&mut dyn Read>) -> Result<String, CommandError> {
    get_from_header(bytes, ' ')
}

// "blob 16\u0000"
fn get_len(bytes: &mut std::io::Bytes<&mut dyn Read>) -> Result<String, CommandError> {
    get_from_header(bytes, '\0')
}

///
fn get_from_header(
    bytes: &mut std::io::Bytes<&mut dyn Read>,
    char_stop: char,
) -> Result<String, CommandError> {
    let type_str = {
        let end = char_stop as u8;
        let mut result = String::new();
        loop {
            if let Some(Ok(byte)) = bytes.next() {
                if byte == end {
                    break;
                }
                result.push(byte as char);
            } else {
                return Err(CommandError::FileReadError(
                    "Error leyendo bytes para obtener el tipo de objeto git".to_string(),
                ));
            }
        }
        Ok(result)
    }?;
    Ok(type_str)
}

impl Clone for GitObject {
    fn clone(&self) -> Self {
        self.clone_object()
    }
}

impl std::fmt::Debug for GitObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
