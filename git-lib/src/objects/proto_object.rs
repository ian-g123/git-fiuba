use super::git_object::GitObjectTrait;
use crate::{
    command_errors::CommandError, objects_database::ObjectsDatabase, utils::aux::get_sha1,
};

#[derive(Clone, Debug)]
pub struct ProtoObject {
    content: Vec<u8>,
    _len: usize,
    type_str: String,
}

impl ProtoObject {
    pub fn new(content: Vec<u8>, len: usize, type_str: String) -> Self {
        ProtoObject {
            content,
            _len: len,
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

    fn content(&mut self, _: Option<&mut ObjectsDatabase>) -> Result<Vec<u8>, CommandError> {
        Ok(self.content.clone())
    }

    fn to_string_priv(&mut self) -> String {
        let string = String::from_utf8_lossy(&self.content);
        string.to_string()
    }

    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        self.write_to(&mut buf, None)?;
        Ok(get_sha1(&buf))
    }

    fn get_info_commit(
        &self,
    ) -> Option<(
        String,
        super::author::Author,
        super::author::Author,
        i64,
        i32,
    )> {
        todo!()
    }

    fn get_path(&self) -> Option<String> {
        todo!()
    }
}

// impl fmt::Display for ProtoObject {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.to_string_priv())
//     }
// }
