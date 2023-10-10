extern crate sha1;
use std::{error::{self}, fmt};

use sha1::{Digest, Sha1};


#[derive(Debug, Clone)]
enum ErrorObject {
    ErrorType,
}

impl error::Error for ErrorObject {}

impl fmt::Display for ErrorObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error Object")
    }
}

pub type GitObjectBox = Box<dyn GitObject>;

pub trait GitObject {
    fn object_type(&self) -> String;

    fn get_sha1(&self, data: Vec<u8>) -> String {
        
        let length = String::from_utf8_lossy(&data);
        let header = format!("{} {}\0", self.object_type(),length);
        let mut store = Vec::new();
        store.extend_from_slice(header.as_bytes());
        store.extend_from_slice(&data);
    
        // Calcula el hash SHA-1 del contenido del archivo
        let mut hasher = Sha1::new();
        hasher.update(&store);
        let hash_result = hasher.finalize();
        // Convierte los bytes del hash en una cadena hexadecimal
        let hex_string = hash_result
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("");
        hex_string
    }
}

pub struct Blob {}

impl GitObject for Blob {
    fn object_type(&self) -> String {
        "blob".to_string()    
    }
}

pub struct Commit {}

impl GitObject for Commit {
    fn object_type(&self) -> String {
        "commit".to_string()
    }
}

pub struct Tree {}

impl GitObject for Tree {
    fn object_type(&self) -> String {
        "tree".to_string()
    }
}

pub struct Tag {}

impl GitObject for Tag {
    fn object_type(&self) -> String {
        "tag".to_string()
    }
}

// pub fn create_object(str_type: &str) -> Result<GitObjectBox, ErrorObject>  {
//     match str_type{
//         "blob" => Ok(create_blob()),
//         "commit" => Ok(create_commit()),
//         "tree" => Ok(create_tree()),
//         "tag" => Ok(create_tag()),
//         _  => Err(ErrorObject::ErrorType),
//     }
// }

fn create_blob() -> Box<Blob> {
    Box::new(Blob{})
}

fn create_tree() -> Box<Tree> {
    Box::new(Tree{})
}

fn create_tag() -> Box<Tag> {
    Box::new(Tag{})
}

fn create_commit() -> Box<Commit> {
    Box::new(Commit{})
}
