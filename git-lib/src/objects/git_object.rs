use crate::{
    changes_controller_components::changes_types::ChangeType, command_errors::CommandError,
    logger::Logger, objects_database::ObjectsDatabase, utils::super_string::u8_vec_to_hex_string,
};

use super::{author::Author, blob::Blob, commit_object::CommitObject, mode::Mode, tree::Tree};
use crate::utils::aux::hex_string_to_u8_vec;
use std::{
    collections::HashMap,
    io::{Read, Write},
};

pub type GitObject = Box<dyn GitObjectTrait>;

pub trait GitObjectTrait {
    fn as_mut_blob(&mut self) -> Option<&mut Blob> {
        None
    }

    fn as_mut_tree(&mut self) -> Option<&mut Tree>;

    /// Devuelve el árbol del objeto si es que corresponde, o sino None
    fn as_tree(&mut self) -> Option<Tree> {
        None
    }

    fn as_mut_commit(&mut self) -> Option<&mut CommitObject> {
        None
    }

    fn clone_object(&self) -> GitObject;

    fn write_to(
        &mut self,
        stream: &mut dyn std::io::Write,
        db: Option<&mut ObjectsDatabase>,
    ) -> Result<(), CommandError> {
        let content = self.content(db)?;
        let type_str = self.type_str();
        write_to_stream_from_content(stream, content, type_str)

        //     let type_str = self.type_str();
        //     let len = content.len();
        //     let header = format!("{} {}\0", type_str, len);
        //     stream
        //         .write(header.as_bytes())
        //         .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        //     stream
        //         .write(content.as_slice())
        //         .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        //     Ok(())
    }

    /// Agrega un árbol al objeto Tree si es que corresponde, o sino un Blob\
    /// Si el objeto no es un Tree, devuelve un error
    fn add_path(
        &mut self,
        logger: &mut Logger,
        _vector_path: Vec<&str>,
        _current_depth: usize,
        _hash: &String,
    ) -> Result<(), CommandError> {
        logger.log("ERROR: No se puede agregar un path a un objeto que no es un árbol");
        Err(CommandError::ObjectNotTree)
    }

    /// Devuelve el tipo del objeto hecho string
    fn type_str(&self) -> String;

    /// Devuelve el modo del objeto
    fn mode(&self) -> Mode;

    /// Devuelve el contenido del objeto
    fn content(&mut self, db: Option<&mut ObjectsDatabase>) -> Result<Vec<u8>, CommandError>;

    /// Devuelve el tamaño del objeto en bytes
    fn size(&mut self, db: Option<&mut ObjectsDatabase>) -> Result<usize, CommandError> {
        let content = self.content(db)?;
        Ok(content.len())
    }

    fn to_string_priv(&mut self) -> String;

    /// Devuelve el hash del objeto
    fn get_hash(&mut self) -> Result<[u8; 20], CommandError>;

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

    fn restore(&mut self, _path: &str, _logger: &mut Logger) -> Result<(), CommandError> {
        Ok(())
    }

    fn checkout_restore(
        &mut self,
        _path: &str,
        _logger: &mut Logger,
        deletions: &mut Vec<String>,
        modifications: &mut Vec<String>,
        conflicts: &mut Vec<String>,
        common: &mut Tree,
        unstaged_files: &Vec<String>,
        staged: &HashMap<String, Vec<u8>>,
    ) -> Result<bool, CommandError> {
        Ok(false)
    }

    fn set_hash(&mut self, _hash: [u8; 20]) {}
}

pub fn display_from_hash(
    db: &ObjectsDatabase,
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = db.read_file(hash, logger)?;

    let mut stream = std::io::Cursor::new(content);
    display_from_stream(&mut stream, logger, output)
}

pub fn display_from_stream(
    stream: &mut dyn Read,
    logger: &mut Logger,
    output: &mut dyn Write,
) -> Result<(), CommandError> {
    let (type_str, len) = get_type_and_len(stream)?;
    if type_str == "blob" {
        return Blob::display_from_stream(stream, len, output, logger);
    }
    if type_str == "tree" {
        return Tree::display_from_stream(stream, len, output, logger);
    };
    if type_str == "commit" {
        return CommitObject::display_from_stream(stream, len, output);
    };
    Err(CommandError::ObjectTypeError)
}

pub fn display_type_from_hash(
    db: &ObjectsDatabase,
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = db.read_file(hash, logger)?;
    let mut stream = std::io::Cursor::new(content);
    let (type_str, _) = get_type_and_len(&mut stream)?;
    writeln!(output, "{}", type_str)
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
    Ok(())
}

pub fn display_size_from_hash(
    db: &ObjectsDatabase,
    output: &mut dyn Write,
    hash: &str,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let (_, content) = db.read_file(hash, logger)?;
    let mut stream = std::io::Cursor::new(content);
    let (_, len) = get_type_and_len(&mut stream)?;
    writeln!(output, "{}", len).map_err(|error| CommandError::FileWriteError(error.to_string()))?;
    Ok(())
}

pub fn read_git_object_from(
    db: &ObjectsDatabase,
    stream: &mut dyn Read,
    path: &str,
    hash_str: &str,
    logger: &mut Logger,
) -> Result<GitObject, CommandError> {
    let (type_str, len) = get_type_and_len(stream)?;

    logger.log(&format!("Reading object of type : {}", type_str));
    if type_str == "blob" {
        let mut blob = Blob::read_from(stream, len, path, hash_str, logger)?;
        let hash_hex = hex_string_to_u8_vec(hash_str);
        blob.set_hash(hash_hex);
        return Ok(blob);
    };
    if type_str == "tree" {
        return Tree::read_from(Some(db), stream, len, path, hash_str, logger);
    };
    if type_str == "commit" {
        return CommitObject::read_from(Some(db), stream, logger, None);
    };

    Err(CommandError::ObjectTypeError)
}

pub fn get_type_and_len(stream: &mut dyn Read) -> Result<(String, usize), CommandError> {
    let mut bytes = stream.bytes();
    let type_str = get_type(&mut bytes)?;
    let len_str = get_string_up_to_null_byte(&mut bytes)?;
    let len: usize = len_str
        .parse()
        .map_err(|_| CommandError::ObjectLengthParsingError)?;
    Ok((type_str, len))
}

fn get_type(bytes: &mut std::io::Bytes<&mut dyn Read>) -> Result<String, CommandError> {
    get_from_header(bytes, ' ')
}

fn get_string_up_to_null_byte(
    bytes: &mut std::io::Bytes<&mut dyn Read>,
) -> Result<String, CommandError> {
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

pub fn write_to_stream_from_content(
    stream: &mut dyn std::io::Write,
    content: Vec<u8>,
    type_str: String,
) -> Result<(), CommandError> {
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
