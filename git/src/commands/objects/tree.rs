use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::commands::command_errors::CommandError;

use super::{mode::Mode, tree_or_blob::TreeOrBlob, aux::*, blob::Blob};

#[derive(Debug, Clone)]
pub struct Tree{
    hash: String,
    mode: Mode,
    path: String,
    name: String,
    objects: HashMap<String, TreeOrBlob>
}


impl Tree{
    pub fn new(path: String, objects: HashMap<String, TreeOrBlob>)-> Result<Self, CommandError>{
        let object_type = "tree";
        let mode = Mode::get_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string(), true)?;
        let objects: HashMap<String, TreeOrBlob> = HashMap::new();
        Ok(Tree{
            hash: sha1,
            mode: mode,
            path: path.clone(),
            name: get_name(&path)?,
            objects: objects,
        })
    }

    pub fn get_hash(&self)->String{
        self.hash.clone()
    }

    fn add_object(&mut self, object: TreeOrBlob){
        let hash_object = object.get_hash();
        _ = self.objects.insert(hash_object, object);
    }

    pub fn add_blob(&mut self, path_name: &String, hash: &String)-> Result<(), CommandError>{
        let path = PathBuf::from(path_name.clone());
        let Some(parent_path) = path.parent() else{
            return Err(CommandError::FileNotFound(path_name.to_owned()))
        };
        let Some(parent_path) = parent_path.to_str()else{
            return Err(CommandError::FileNotFound(path_name.to_owned()))
        };
        if self.path == parent_path{
            let blob = Blob::new_from_hash(hash.to_owned(), path_name.to_owned())?;
            _ = self.objects.insert(blob.get_hash(), TreeOrBlob::BlobObject(blob));
        } else{
            self.add_blob_aux(path_name, &hash)?;
        }
        Ok(())
    }

    fn add_blob_aux(&mut self, path_name: &String, hash: &String)-> Result<(), CommandError>{
        for (_, object) in self.objects.iter_mut(){
            if let TreeOrBlob::TreeObject(mut tree) = object.clone(){
                tree.add_blob(path_name, hash)?;
                *object = TreeOrBlob::TreeObject(tree);
            }
        }
        Ok(())
    }

}