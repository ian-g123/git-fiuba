use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::commands::command_errors::CommandError;

use super::{
    aux::*,
    blob::Blob,
    mode::Mode,
    tree_or_blob::{TreeLike, TreeOrBlobTrait},
};

//#[derive(Debug, Clone)]
#[derive(Clone)]
pub struct Tree {
    hash: String,
    mode: Mode,
    path: String,
    name: String,
    objects: HashMap<String, TreeLike>,
}

impl Tree {
    /// Crea un Tree a partir de su ruta y los objetos alos que referencia. Si la ruta no existe,
    ///  devuelve Error.
    pub fn new(path: String, objects: HashMap<String, TreeLike>) -> Result<Self, CommandError> {
        let object_type = "tree";
        let mode = Mode::get_mode(path.clone())?;
        let sha1 = get_sha1(path.clone(), object_type.to_string(), true)?;
        let objects: HashMap<String, TreeLike> = HashMap::new();
        Ok(Tree {
            hash: sha1,
            mode: mode,
            path: path.clone(),
            name: get_name(&path)?,
            objects: objects,
        })
    }

    /// Devuelve el hash del Tree.
    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let path = PathBuf::from(path_name.clone());
        let Some(parent_path) = path.parent() else {
            return Err(CommandError::FileNotFound(path_name.to_owned()));
        };
        let Some(parent_path) = parent_path.to_str() else {
            return Err(CommandError::FileNotFound(path_name.to_owned()));
        };
        if self.path == parent_path {
            let blob = Blob::new_from_hash(hash.to_owned(), path_name.to_owned())?;
            _ = self.objects.insert(blob.get_hash(), Box::new(blob));
        } else {
            self.add_blob_aux(path_name, &hash)?;
        }
        Ok(())
    }

    /// Agrega un Objeto Blob o Tree al Tree.
    fn add_object(&mut self, object: TreeLike) {
        let hash_object = object.get_hash();
        _ = self.objects.insert(hash_object, object);
    }

    /// Busca el Tree donde debe guardarse el Blob.
    fn add_blob_aux(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        for (_, object) in self.objects.iter_mut() {
            let Some(mut tree) = object.as_mut_tree() else {
                continue;
            };
            tree.add_blob(path_name, hash)?;
        }
        Ok(())
    }
}

impl TreeOrBlobTrait for Tree {
    fn get_hash(&self) -> String {
        self.hash.clone()
    }

    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn clone_object(&self) -> TreeLike {
        Box::new(self.clone())
    }
}
