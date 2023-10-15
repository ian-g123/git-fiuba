use std::collections::HashMap;

use crate::commands::command_errors::CommandError;

use super::{mode::Mode, tree_or_blob::TreeOrBlob, aux::*};

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
        let mode = set_mode(path.clone())?;
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
}