extern crate sha1;
use sha1::{Digest, Sha1};

pub trait TypeObject {
    fn get_sha1(&self, data: &str) -> String;
}

pub struct Blob {}

impl Blob {
    pub fn get_sha1(data: String) -> String {

        

        return get_sha1_aux(data);
    }
}

pub struct Commit {}

impl Commit {
    pub fn get_sha1(data: String) -> String {
        return get_sha1_aux(data);
    }
}

pub struct Tree {}

impl Tree {
    pub fn get_sha1(data: String) -> String {
        return get_sha1_aux(data);
    }
}

pub struct Tag {}

impl Tag {
    pub fn get_sha1(data: String) -> String {
        return get_sha1_aux(data);
    }
}

fn get_sha1_aux(data: String) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());

    // Obtener el resultado del hash como bytes
    let result = hasher.finalize();

    // Convierte el resultado a una representación hexadecimal
    format!("{:x}", result)
}
