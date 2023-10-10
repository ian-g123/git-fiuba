use std::{fs::File, error::Error};

use super::type_values::{Blob, Commit, Tag, Tree};
use crate::error_args::ErrorArgs;

pub enum FlagsHashObject {
    Type,
    Write,
    StdIn,
    Path(String),
    StdinPaths,
}

impl FlagsHashObject {
    pub fn get_flag(flag: &str, values: Vec<String>) -> Result<String, ErrorArgs> {
        match flag {
            "-t" => create_type(values),
            "-w" => create_write(values),
            "--stdin" => create_stdin(values),
            "--path" => create_path(values),
            "--stdin-paths" => create_stdin_paths(values),
            _ => Err(ErrorArgs::InvalidFlag),
        }
    }
}

fn create_type(values: Vec<String>) -> Result<String, Box<dyn Error>> {
    if values.len() != 2 {
        return Err(ErrorArgs::InvalidFlag);
    }

    let mut content = Vec::new();
    let mut data = values[1].clone();
    
    let mut file = File::open(data)?;
    data = file.read_to_end(&mut content)?;
    
    match values.first().map(|s| s.as_str()) {
        Some("blob") => return Ok(Blob::get_sha1(values[1])),
        Some("commit") => return Ok(Commit::get_sha1(values[1])),
        Some("tag") => return Ok(Tag::get_sha1(values[1])),
        Some("tree") => return Ok(Tree::get_sha1(values[1])),
        _ => return Err(ErrorArgs::InvalidFlag),
    }
}

fn create_write(values: Vec<String>) -> Result<String, ErrorArgs> {
    if values.len() != 0 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok("String".to_string())
}

fn create_stdin(values: Vec<String>) -> Result<String, ErrorArgs> {
    if values.len() != 0 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok("String".to_string())
}

fn create_stdin_paths(values: Vec<String>) -> Result<String, ErrorArgs> {
    if values.len() != 0 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok("String".to_string())
}

fn create_path(values: Vec<String>) -> Result<String, ErrorArgs> {
    if values.len() != 1 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok("String".to_string())
}
