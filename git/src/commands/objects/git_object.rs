use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{
    author::Author, aux::get_sha1, blob::Blob, commit_object::CommitObject, mode::Mode,
    super_string::u8_vec_to_hex_string, tree::Tree,
};
use std::{
    fmt,
    io::{Cursor, Read, Write},
};

pub type GitObject = Box<dyn GitObjectTrait>;

pub trait GitObjectTrait {
    fn as_mut_tree(&mut self) -> Option<&mut Tree>;

    /// Devuelve el árbol del objeto si es que corresponde, o sino None
    fn as_tree(&mut self) -> Option<Tree> {
        None
    }

    fn as_commit_mut(&mut self) -> Option<&mut CommitObject> {
        None
    }

    fn clone_object(&self) -> GitObject;

    fn write_to(&mut self, stream: &mut dyn std::io::Write) -> Result<(), CommandError> {
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
        logger: &mut Logger,
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
    fn content(&mut self) -> Result<Vec<u8>, CommandError>;

    /// Devuelve el tamaño del objeto en bytes
    fn size(&self) -> Result<usize, CommandError> {
        let content = self.to_string_priv();
        Ok(content.len())
    }

    fn to_string_priv(&self) -> String;

    /// Devuelve el hash del objeto
    fn get_hash(&mut self) -> Result<[u8; 20], CommandError>; /*  {
                                                                  let mut buf: Vec<u8> = Vec::new();
                                                                  let mut stream = Cursor::new(&mut buf);
                                                                  self.write_to(&mut stream)?;
                                                                  Ok(get_sha1(&buf))
                                                              } */

    /// Devuelve el hash del objeto
    fn get_hash_string(&mut self) -> Result<String, CommandError> {
        Ok(u8_vec_to_hex_string(&self.get_hash()?))
    }

    ///
    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)>;

    fn get_path(&self) -> Option<String>;

    fn get_name(&self) -> Option<String> {
        None
    }
}

pub fn display_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    logger.log(&format!("About to read file, hash = {}\n", hash));
    let (_, content) = objects_database::read_file(hash, logger)?;
    logger.log("file read\n");

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
    if type_str == "commit" {
        return CommitObject::display_from_stream(stream, len, output, logger);
    };
    Err(CommandError::ObjectTypeError)
}

pub fn display_type_from_hash(
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = objects_database::read_file(hash, logger)?;
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
    let (_, content) = objects_database::read_file(hash, logger)?;
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
    logger.log("Reading git object...");

    let (type_str, len) = get_type_and_len(stream, logger)?;

    logger.log(&format!("len: {}, type: {}", len, type_str));

    if type_str == "blob" {
        return Blob::read_from(stream, len, path, hash_str, logger);
    };
    if type_str == "tree" {
        return Tree::read_from(stream, len, path, hash_str, logger);
    };
    if type_str == "commit" {
        return CommitObject::read_from(stream, logger);
    };

    Err(CommandError::ObjectTypeError)
}

fn get_type_and_len(
    stream: &mut dyn Read,
    logger: &mut Logger,
) -> Result<(String, usize), CommandError> {
    let mut bytes = stream.bytes();
    let type_str = get_type(&mut bytes)?;
    let len_str = get_len(&mut bytes)?;
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

// impl std::fmt::Debug for GitObject {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.to_string())
//     }
// }
