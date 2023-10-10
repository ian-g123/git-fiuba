use std::{error::Error, fs::File, io::Read};

// use super::type_values::create_object;
use crate::error_args::ErrorFlags;

pub enum FlagsHashObject {
    Type,
    Write,
    StdIn,
    Path(String),
    StdinPaths,
}

impl FlagsHashObject {
    pub fn get_flag(flag: &str, values: Vec<String>) -> Result<FlagsHashObject, Box<dyn Error>> {
        match flag {
            "-t" => create_type(values),
            // "-w" => create_write(values),
            // "--stdin" => create_stdin(values),
            // "--path" => create_path(values),
            // "--stdin-paths" => create_stdin_paths(values),
            _ => Err(Box::new(ErrorFlags::InvalidFlag)),
        }
    }

    fn calculate_sha1(values: Vec<String>) -> Result<String, Box<dyn Error>> {
        if values.len() != 2 {
            return Err(Box::new(ErrorFlags::InvalidFlag));
        }
    
        let git_object_str = values[0].clone();
        let git_object = create_object(git_object_str.as_str())?;
        
        let mut file = File::open(values[1])?;
        let mut content = Vec::new();
        let bytes_read = file.read_to_end(&mut content)?;
    
        Ok(git_object.get_sha1(content))
    }
}

fn create_type(values: Vec<String>) -> Result<FlagsHashObject, Box<dyn Error>> {
    if values.len() != 1 {
        return Err(Box::new(ErrorFlags::InvalidFlag));
    }
    Ok(FlagsHashObject::Type{})
}

fn create_stdin(values: Vec<String>) -> Result<String, ErrorFlags> {
    if values.len() != 0 {
        return Err(ErrorFlags::InvalidFlag);
    }
    Ok("String".to_string())
}

fn create_stdin_paths(values: Vec<String>) -> Result<String, ErrorFlags> {
    if values.len() != 0 {
        return Err(ErrorFlags::InvalidFlag);
    }
    Ok("String".to_string())
}

fn create_path(values: Vec<String>) -> Result<String, ErrorFlags> {
    if values.len() != 1 {
        return Err(ErrorFlags::InvalidFlag);
    }
    Ok("String".to_string())
}


/* Hash:  
-40 caracteres 
-Header: "<tipo_obj> #{content.length}\u0000"

hash = sha1(content + header) */