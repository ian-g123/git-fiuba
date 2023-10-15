use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::commands::command_errors::CommandError;

use super::{
    aux::*,
    blob::Blob,
    git_object::{GitObject, GitObjectTree},
    mode::Mode,
};

//#[derive(Debug, Clone)]
#[derive(Clone)]
pub struct Tree {
    mode: Mode,
    path: String,
    // name: String,
    objects: HashMap<String, GitObject>,
}

impl Tree {
    /// Crea un Tree a partir de su ruta y los objetos a los que referencia. Si la ruta no existe,
    /// devuelve Error.
    pub fn new(path: String, objects: HashMap<String, GitObject>) -> Result<Self, CommandError> {
        let object_type = "tree";
        let mode = Mode::get_mode(path.clone())?;
        // let sha1 = get_sha1(path.clone(), object_type.to_string(), true)?;
        let objects: HashMap<String, GitObject> = HashMap::new();

        Ok(Tree {
            mode: mode,
            path: path.clone(),
            // name: get_name(&path)?,
            objects: objects,
        })
    }

    pub fn new_from_path(path: &str) -> Self {
        let objects: HashMap<String, GitObject> = HashMap::new();

        Tree {
            mode: Mode::get_mode(path.to_string()).unwrap(),
            path: path.to_string(),
            // name: get_name(path).unwrap(),
            objects: objects,
        }
    }

    /// Devuelve el hash del Tree.
    pub fn get_hash_interno(&self) -> String {
        todo!()
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let parent_path = get_parent(path_name)?;

        if self.path == parent_path {
            let blob = Blob::new_from_hash(hash.to_owned(), path_name.to_owned())?;
            _ = self.objects.insert(path_name.to_string(), Box::new(blob));
            return Ok(());
        }
        if parent_path.starts_with(&self.path) {
            return self.add_blob_to_subtree(path_name, &hash);
        }
        Err(CommandError::NotYourFather)
    }

    /// Agrega un Objeto Blob o Tree al Tree.
    fn insert(&mut self, path: &str, object: GitObject) {
        _ = self.objects.insert(path.to_string(), object);
    }

    /// Busca el Tree donde debe guardarse el Blob.
    fn add_blob_to_subtree(
        &mut self,
        path_name: &String,
        hash: &String,
    ) -> Result<(), CommandError> {
        for (_, object) in self.objects.iter_mut() {
            let Some(mut tree) = object.as_mut_tree() else {
                continue;
            };
            match tree.add_blob(path_name, hash) {
                Ok(()) => return Ok(()),
                Err(CommandError::NotYourFather) => continue,
                Err(error) => return Err(error),
            };
        }
        let child_tree = self.add_new_blob_in_new_tree(path_name, &hash)?;
        self.insert(path_name, Box::new(child_tree));
        Ok(())
    }

    fn add_new_blob_in_new_tree(&self, path_name: &str, hash: &str) -> Result<Tree, CommandError> {
        let parent_path = get_parent(path_name)?;
        let blob = Blob::new_from_hash(hash.to_string(), path_name.to_string())?;
        let mut tree = Tree::new_from_path(&parent_path);
        tree.insert(path_name, Box::new(blob));
        self.add_tree_in_new_tree(&parent_path, tree)
    }

    fn add_tree_in_new_tree(&self, path_name: &str, tree: Tree) -> Result<Tree, CommandError> {
        let parent_path = get_parent(path_name)?;
        if parent_path == self.path {
            return Ok(tree);
        }
        let mut parent_tree = Tree::new_from_path(&parent_path);
        parent_tree.insert(path_name, Box::new(tree));
        self.add_tree_in_new_tree(&parent_path, parent_tree)
    }
}

fn get_parent(path_name: &str) -> Result<String, CommandError> {
    let path: PathBuf = PathBuf::from(path_name);
    match path.parent() {
        Some(parent_path) => match parent_path.to_str() {
            Some(parent_path) => Ok(parent_path.to_owned()),
            None => return Err(CommandError::FileNotFound(path_name.to_owned())),
        },
        None => return Err(CommandError::FileNotFound(path_name.to_owned())),
    }
}

impl GitObjectTree for Tree {
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "tree".to_string()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        todo!()
    }
}
