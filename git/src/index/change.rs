// Se deben crear con 'git add' y agregar al Index con 'git commit'.

use std::{path::PathBuf, io::Read, fs::{File, self}};
use std::os::unix::fs::PermissionsExt;

use crate::commands::{command_errors::CommandError, file_compressor::{extract, compress}};

use super::mode::Mode;

#[derive(Clone)]
pub struct Change{
    path: String,
    hash: String,
    content: Vec<u8>,
    mode: Mode,
}

impl Change {
    pub fn new(path: String, hash:String)-> Result<Self, CommandError>{
        let mut data : Vec<u8> = Vec::new();
        let Ok(mut file) = File::open(path.clone()) else{
            return Err(CommandError::FileOpenError(path));
        };
        let Ok(content) = file.read_to_end(&mut data) else{
            return Err(CommandError::FileReadError(path))
        };
        let Ok(content) = compress(&data) else{
            return Err(CommandError::CompressionError)
        };
        let mode = Self::set_mode(path.clone())?;
            Ok(Change { path: path, hash: hash, content: content, mode:mode })
    }

    pub fn get_mode(&self)->usize{
        self.mode.get_mode().clone()
    }

    fn set_mode(path: String)->Result<Mode, CommandError>{
        let mode: Mode;
        let Ok(metadata) = fs::metadata(path.clone()) else{
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

    pub fn get_path(&self)-> String{
        self.path.clone()
    }

    pub fn get_hash(&self)-> String{
        self.hash.clone()
    }

    pub fn get_content(&self)->Result<Vec<u8>, CommandError>{
        Ok(extract(&self.content)?)
    }   

}
