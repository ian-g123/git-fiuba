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
    let type_str = {
        let end = ' ' as u8;
        let mut result = String::new();
        let Some(Ok(mut byte)) = bytes.next() else {
            return Err(CommandError::FileReadError(
                "Error leyendo bytes 1".to_string(),
            ));
        };
        while byte != end {
            result.push(byte as char);
            let Some(Ok(byte_h)) = bytes.next() else {
                return Err(CommandError::FileReadError(
                    "Error leyendo bytes 2".to_string(),
                ));
            };
            byte = byte_h;
        }
        Ok(result)
    }?;
    let len_str = {
        let mut result = String::new();
        let Some(Ok(mut byte)) = bytes.next() else {
            return Err(CommandError::FileReadError(
                "Error leyendo bytes 3".to_string(),
            ));
        };
        let end = '\0' as u8;
        while byte != end {
            result.push(byte as char);
            let Some(Ok(byte_h)) = bytes.next() else {
                return Err(CommandError::FileReadError(
                    "Error leyendo bytes 4".to_string(),
                ));
            };
            byte = byte_h;
        }
        Ok(result)
    }?;
    logger.log("found \0");
    let len: usize = len_str
        .parse()
        .map_err(|_| CommandError::ObjectLengthParsingError)?;
    Ok((type_str, len))
}

// impl fmt::Display for GitObject {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.to_string())
//     }
// }

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
