use std::fmt;

use super::tree::Tree;

pub type TreeLike = Box<dyn TreeOrBlobTrait>;

pub trait TreeOrBlobTrait {
    fn get_hash(&self) -> String;

    fn as_mut_tree(&mut self) -> Option<&mut Tree>;

    fn clone_object(&self) -> TreeLike;
}

impl fmt::Display for TreeLike {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_hash())
    }
}

impl Clone for TreeLike {
    fn clone(&self) -> Self {
        self.clone_object()
    }
}
