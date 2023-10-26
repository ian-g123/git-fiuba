use std::fmt;

use super::git_object::GitObjectTrait;

#[derive(Clone, Debug)]
pub struct ProtoObject {
    content: Vec<u8>,
    len: usize,
    type_str: String,
}

impl ProtoObject {
    pub fn new(content: Vec<u8>, len: usize, type_str: String) -> Self {
        ProtoObject {
            content,
            len,
            type_str,
        }
    }
}

impl GitObjectTrait for ProtoObject {
    fn as_mut_tree(&mut self) -> Option<&mut super::tree::Tree> {
        None
    }

    fn clone_object(&self) -> super::git_object::GitObject {
        todo!()
    }

    fn type_str(&self) -> String {
        self.type_str.clone()
    }

    fn mode(&self) -> super::mode::Mode {
        todo!()
    }

    fn content(&mut self) -> Result<Vec<u8>, crate::commands::command_errors::CommandError> {
        Ok(self.content.clone())
    }

    fn to_string_priv(&self) -> String {
        let string = String::from_utf8_lossy(&self.content);
        string.to_string()
    }
}

impl fmt::Display for ProtoObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_priv())
    }
}
