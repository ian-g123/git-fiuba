use std::io::Read;
use std::os::unix::prelude::PermissionsExt;
use std::{collections::HashMap, os::unix::fs};
use std::fs::{self as _fs, File};

extern crate sha1;
use sha1::{Digest, Sha1};

use crate::commands::command_errors::CommandError;
use crate::commands::hash_object_components::hash_object::HashObject;

#[derive(Clone)]
pub struct Commit{
    header:String,
    hash: String,
    mode: Mode,
    parent: Box<Commit>, //cambiar en Merge (puede tener varios padres),
    message: String,
    author: Author,
    date: String, //cambiar por Date o TimeStamp
}

#[derive(Clone)]
pub struct Author{
    name: String,
    email: String,
}

pub struct Tree{
    hash: String,
    mode: Mode,
    path: String,
    name: String,
    objects: HashMap<String, TreeOrBlob>
}

#[derive(Clone)]
pub enum Mode{
    RegularFile = 100644,
    ExecutableFile = 100755,
    SymbolicLink = 120000,
    Submodule = 160000,
    Tree = 040000,

}

#[derive(Clone)]
pub struct Blob{
    mode: Mode,
    path: String,
    hash: String,
    name: String
}

pub enum TreeOrBlob{
    TreeObject(Tree),
    BlobObject(Blob)
}

impl TreeOrBlob{
    fn get_hash(&self)-> String{
        match self{
            TreeOrBlob::BlobObject(blob) => blob.get_hash(),
            TreeOrBlob::TreeObject(tree) => tree.get_hash()
        }
    }
}

impl Blob{
    fn new(path: String)-> Result<Self, CommandError>{
        let object_type = "blob";
        let mode = set_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string())?;
        Ok(Blob{
            mode: mode, path: path.clone(),
            name: get_name(path), hash:sha1
        })
    }

    fn get_hash(&self)->String{
        self.hash.clone()
    }
}

impl Tree{
    fn new(path: String)-> Result<Self, CommandError>{
        let object_type = "tree";
        let mode = set_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string())?;
        let objects: HashMap<String, TreeOrBlob> = HashMap::new();
        Ok(Tree{
            hash: sha1,
            mode: mode,
            path: path.clone(),
            name: get_name(path),
            objects: objects,
        })
    }

    fn get_hash(&self)->String{
        self.hash.clone()
    }

    fn add_object(&mut self, object: TreeOrBlob){
        let hash_object = object.get_hash();
        _ = self.objects.insert(hash_object, object);
    }
}

fn get_name(path: String)-> String{
    let parts: Vec<&str> = path.split('/').collect();
    parts[parts.len()-1].to_string()
}

fn get_sha1(path: String, object_type: String) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, false, false);
    let (hash, _) = hash_object.run_for_content(content)?;
    Ok(hash)
}


fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

fn set_mode(path: String)->Result<Mode, CommandError>{
    let mode: Mode;
    let Ok(metadata) = _fs::metadata(path.clone()) else{
        return Err(CommandError::FileNotFound(path));
    };
    let permissions_mode= metadata.permissions().mode();
    if metadata.is_dir(){
        mode = Mode::Tree;
    } else if metadata.is_symlink(){
        mode = Mode::SymbolicLink;
    } else if (permissions_mode & 0o111) != 0{
        mode = Mode::ExecutableFile;
    }else{
        mode = Mode::RegularFile;
    }
    Ok(mode)
}