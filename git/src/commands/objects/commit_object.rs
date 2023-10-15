use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, self};
use std::io::Read;
use std::path::{PathBuf, Path};

use crate::commands::file_compressor::extract;
use crate::commands::{stagin_area::StagingArea, command_errors::CommandError};

use super::aux::get_sha1;
use super::blob::Blob;
use super::tree_or_blob::TreeOrBlob;
use super::{author::Author, mode::Mode, tree::Tree};

extern crate chrono;
use chrono::{prelude::*, TimeZone};
use sha1::digest::block_buffer::Error;

#[derive(Clone)]
pub struct Commit{
    //hash: String,
    parent: Option<String>, //cambiar en Merge (puede tener varios padres),
    message: String,
    author: Author,
    date: String, //cambiar por Date o TimeStamp
    tree: Tree, 
}

impl Commit{
    pub fn new(index: StagingArea,message: String, author: Author)-> Result<(), CommandError>{
        let mut parent: Option<String> = None;
        let parent_hash = Commit::get_parent()?;
        if !parent_hash.is_empty(){
            parent = Some(parent_hash)
        }
        let timestamp = Commit::get_timestamp();

        let tree = CommitTree::new(index.files, parent)?;

        // falta hash
        
        Ok(())
    }

    pub fn change_date(&mut self, date: String){
        self.date = date;
    }

    fn get_timestamp(){
        let timestamp: DateTime<Local> = Local::now();
        // formateo para que se vea como el de git.
        timestamp.format("%a %b %e %H:%M:%S %Y %z").to_string();
    }

    fn get_parent()-> Result<String, CommandError>{
        let mut branch = String::new();
        let mut parent = String::new();
        let path = ".git/HEAD";
        let Ok(mut head) = File::open(path) else{
            return Err(CommandError::NotGitRepository);
        };

        if head.read_to_string(&mut branch).is_err(){
            return Err(CommandError::FileReadError(path.to_string()));
        }
        let branch = branch.trim();
        let Some(branch) = branch.split(" ").last() else{
            return Err(CommandError::HeadError);
        };
        let branch_path = format!(".git/{}", branch);
        let Ok(mut branch_file) = File::open(path) else{
            return Err(CommandError::FileOpenError(branch_path));
        };
        if branch_file.read_to_string(&mut parent).is_err(){
            return Err(CommandError::FileReadError(branch_path.to_string()));
        }
        let parent = parent.trim();
        Ok(parent.to_string())
    }

    
}

fn get_current_dir()-> Result<PathBuf, CommandError>{
    let Ok(current_dir) = current_dir() else{
        return Err(CommandError::NotGitRepository);
    };
    Ok(current_dir)
}

struct CommitTree{
    objects: HashMap<String, TreeOrBlob> 
}

impl CommitTree{
    fn new(index: HashMap<String, String>, parent: Option<String>)-> Result<Tree, CommandError>{
        let path = get_current_dir()?;
        //let tree = Tree::new(path)?;
        let path_name = Self::get_path_name(path)?;
        let mut objects:HashMap<String, TreeOrBlob> = HashMap::new();
        Self::compare(path_name.clone(), &index, &mut objects, &parent)?;
        let tree = Self::create_tree(&path_name, objects)?;
        Ok(tree)
    }

    fn get_current_dir()-> Result<PathBuf, CommandError>{
        let Ok(current_dir) = current_dir() else{
            return Err(CommandError::NotGitRepository);
        };
        Ok(current_dir)
    }

    pub fn compare(path_name: String, index: &HashMap<String, String>, objects:&mut HashMap<String, TreeOrBlob>, parent:&Option<String>)-> Result<(), CommandError>{
        let path = Path::new(&path_name); 

        let Ok(entries) = fs::read_dir(path.clone()) else{
            return Err(CommandError::InvalidDirectory);
        };
        for entry in entries {
            let Ok(entry) = entry else{
                return Err(CommandError::InvalidDirectoryEntry);
            };
            let entry_path = entry.path();
            let entry_name = Self::get_path_name(entry_path.clone())?;

            if entry_path.is_dir() {

                let mut objects: HashMap<String, TreeOrBlob> = HashMap::new();
                Self::compare(entry_name.clone(), index, &mut objects, parent)?;
                if !index.is_empty(){
                    let tree = Self::create_tree(&entry_name, objects.to_owned())?;
                    _ = objects.insert(tree.get_hash(),TreeOrBlob::TreeObject(tree));
                    return Ok(());
                }

            } else{
                let result = Self::compare_entry(&path_name, index, parent)?;
                if let Some(blob) = result{
                    _ = objects.insert(blob.get_hash(), TreeOrBlob::BlobObject(blob));
                }
                
            }
            
        } 
        Ok(())
    }

    fn create_tree(path:&String, objects:HashMap<String, TreeOrBlob>)-> Result<Tree, CommandError>{
        Ok(Tree::new(path.to_owned(),objects)?)      
    }

    fn compare_entry(path: &String, index: &HashMap<String, String>, parent: &Option<String>)->Result<Option<Blob>, CommandError>{
        let mut blob:Blob;
        if index.contains_key(path){
            let Some(hash) = index.get(path) else{
                return Err(CommandError::FileNotFound(path.to_string()));
            };
            blob = Blob::new_from_hash(hash.to_string(), path.to_owned())?;
            return Ok(Some(blob));
        }
        let hash = get_sha1(path.to_owned(), "blob".to_string(), false)?;

        if let Some(parent)= parent{
            let found = Self::search_parent_commit(parent.to_string(), hash)?;
            if found{
                blob = Blob::new(path.to_owned())?;
                return Ok(Some(blob));
            }
        }
    
        Ok(None)
        
    }

    fn read_content(hash: String)-> Result<Vec<u8>, CommandError>{
        let mut data :Vec<u8> = Vec::new();
        let path = format!(".git/objects/{}/{}", hash[..2].to_string(), hash[2..].to_string() );
        let Ok(mut tree_file) = File::open(&path) else{
            return Err(CommandError::FileNotFound(path));
        };
        if tree_file.read_to_end(&mut data).is_err(){
            return Err(CommandError::FileReadError(path));
        }
        Ok(data)
    }

    fn search_parent_commit(parent_hash:String, blob_hash: String)-> Result<bool, CommandError>{
        let path = format!(".git/objects/{}/{}", parent_hash[..2].to_string(), parent_hash[2..].to_string() );
        let data = Self::read_content(parent_hash)?;
        let data = extract(&data)?;
        let buf = String::from_utf8_lossy(&data).to_string();
        let lines: Vec<&str> = buf
        .split_terminator("\n")
        .collect();
        for line in lines{
            let info: Vec<&str>= line.split_terminator(" ").collect();
            let (obj_type, obj_hash) = (info[1], info[2]);
            if obj_hash == blob_hash{
                return Ok(true);
            }
            if obj_type == "tree"{
                return Self::search_parent_commit(obj_hash.to_string(), blob_hash);
            }
            
        }
        Ok(false)
    }

    fn get_path_name(path: PathBuf)->Result<String, CommandError>{
        let Some(path_name) = path.to_str() else{
            return Err(CommandError::InvalidDirectoryEntry);
        };
        Ok(path_name.to_string())
    }
}

#[cfg(test)]
mod test{
    use std::io::{self, Write};

    use crate::commands::file_compressor::compress;

    use super::*;
    /* #[test]
    fn timestamp(){
        Commit::get_timestamp();
        assert!(false)
    } */

    fn write()->Result<(), CommandError>{
        let Ok(mut file) = File::create(".git/objects/e3/540872766f87b1de467a5e867d656a6e6fe959") else{
            return Err(CommandError::CompressionError);
        };

        // Contenido que deseas escribir en el archivo
        let contenido = "100644 blob 09c857543fc52cd4267c3825644b4fd7f437dc3f .gitignore\n040000 tree d3a471637c78c8f67cca05221a942bd7efabb58c git".as_bytes();
        let contenido = compress(&contenido)?;

        // Escribe el contenido en el archivo
        if file.write_all(&contenido).is_err(){
            return Err(CommandError::CompressionError);
        }      

        //

        let Ok(mut file) = File::create(".git/objects/d3/a471637c78c8f67cca05221a942bd7efabb58c") else{
            return Err(CommandError::CompressionError);
        };

        // Contenido que deseas escribir en el archivo
        let contenido = "100644 blob f0e37a3b70089bf8ead6970f2d4339527dc628a Cargo.lock\n100644 blob 5da01b81e6f2c1926d9e6df32dc160dfe5326239 Cargo.toml".as_bytes();
        let contenido = compress(&contenido)?;

        // Escribe el contenido en el archivo
        if file.write_all(&contenido).is_err(){
            return Err(CommandError::CompressionError);
        }
        Ok(())
    }

    #[test]
    fn search_parent_commit_test(){
        if write().is_err(){
            assert!(false, "Falló el write");
        }
        assert!(matches!(CommitTree::search_parent_commit("e3540872766f87b1de467a5e867d656a6e6fe959".to_string(), "5da01b81e6f2c1926d9e6df32dc160dfe5326239".to_string()), Ok(true)));
    
    }
}