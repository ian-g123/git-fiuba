use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{aux::get_sha1, blob::Blob, mode::Mode, tree::Tree};
use std::{
    fmt,
    io::{Cursor, Read, Write},
};

pub type GitObject = Box<dyn GitObjectTrait>;

pub trait GitObjectTrait: fmt::Display {
    fn as_mut_tree(&mut self) -> Option<&mut Tree>;

    /// Devuelve el árbol del objeto si es que corresponde, o sino None
    fn as_tree(&self) -> Option<&Tree> {
        None
    }

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

    /// Agrega un árbol al objeto Tree si es que corresponde, o sino un Blob\
    /// Si el objeto no es un Tree, devuelve un error
    fn add_path(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        Err(CommandError::ObjectNotTree)
    }

    /// Devuelve el tipo del objeto hecho string
    fn type_str(&self) -> String;

    /// Devuelve el modo del objeto
    fn mode(&self) -> Mode;

    /// Devuelve el contenido del objeto
    fn content(&self) -> Result<Vec<u8>, CommandError>;

    /// Devuelve el tamaño del objeto en bytes
    fn size(&self) -> Result<usize, CommandError> {
        let content = self.to_string_priv();
        Ok(content.len())
    }

    fn to_string_priv(&self) -> String;

    /// Devuelve el hash del objeto
    fn get_hash(&self) -> Result<[u8; 20], CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(&mut buf);
        self.write_to(&mut stream)?;
        Ok(get_sha1(&buf))
    }
}

pub fn display_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (path, content) = objects_database::read_file(hash)?;
    let mut stream = std::io::Cursor::new(content);
    display_from_stream(&mut stream, logger, output)
}

pub fn display_from_stream(
    stream: &mut dyn Read,
    logger: &mut Logger,
    output: &mut dyn Write,
) -> Result<(), CommandError> {
    let (type_str, len) = get_type_and_len(stream, logger)?;
    if type_str == "blob" {
        return Blob::display_from_stream(stream, len, output, logger);
    }
    if type_str == "tree" {
        return Tree::display_from_stream(stream, len, output, logger);
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
