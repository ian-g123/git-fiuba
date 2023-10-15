use crate::commands::{command::Command, command_errors::CommandError};

use super::{mode::Mode, tree::Tree};
use std::fmt;
pub type GitObject = Box<dyn GitObjectTree>;

pub trait GitObjectTree {
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
}

impl fmt::Display for GitObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "todo")
    }
}

impl Clone for GitObject {
    fn clone(&self) -> Self {
        self.clone_object()
    }
}
