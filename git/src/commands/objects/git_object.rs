use crate::{
    commands::{command::Command, command_errors::CommandError, commit_components::commit::Commit},
    logger::Logger,
};

use super::{blob::Blob, mode::Mode, tree::Tree};
use std::{
    fmt,
    io::{Read, Write},
};
pub type GitObject = Box<dyn GitObjectTrait>;

pub trait GitObjectTrait {
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
        let content = self
            .content()
            .map_err(|error| CommandError::FailToCalculateObjectSize)?;
        Ok(content.len())
    }

    fn to_string(&self) -> &str;
}

pub fn read_git_object_from(
    stream: &mut dyn Read,
    path: &str,
    hash_str: &str,
    logger: &mut Logger,
) -> Result<GitObject, CommandError> {
    let fns = [
        Blob::read_from,
        Tree::read_from,
        // Commit::read_from,
        // Tag::read_from,
    ];

    for read_from in fns {
        match read_from(stream, path, hash_str, logger) {
            Ok(git_object) => return Ok(git_object),
            Err(CommandError::ObjectTypeError) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(CommandError::ObjectTypeError)
}

impl fmt::Display for GitObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Clone for GitObject {
    fn clone(&self) -> Self {
        self.clone_object()
    }
}
