use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;

/// Se obtiene el nombre de los archivos y directorios del workspace\
/// Útil para cuando tengamos que hacer la interfaz de Stage Area
fn get_files_names() -> HashSet<String> { // TODO: remover los unwrap
    let mut files_names = HashSet::new();

    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();

        let file_name = entry.file_name().to_str().unwrap().to_string();
        let directory_name = entry.path().to_str().unwrap().to_string();

        files_names.insert(file_name);
    }
    files_names
}

pub struct StagingArea {
    stagin_area: Tree,
    pointers: HashMap<String, String>,
}

pub struct Tree {
    file_system: HashMap<String, SavedObjects>,
}

enum SavedObjects {
    Blob(String),
    Tree(Tree),
}

impl StagingArea {
    pub fn add(&mut self, file_name: String) {
        self.stagin_area.add_file(file_name);
    }

    pub fn new() -> Self {
        StagingArea {
            stagin_area: Tree::new(),
            pointers: HashMap::new(),
        }
    }
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            file_system: HashMap::new(),
        }
    }
    
    pub fn add_file(&mut self, file_name: String) {
        let file = SavedObjects::Blob(file_name.clone());
        self.file_system.insert(file_name, file);
    }

    pub fn add_tree(&mut self, folder_name: String) {
        let tree = SavedObjects::Tree(Tree::new());
        self.file_system.insert(folder_name, tree);
    }

    pub fn hash(&self) -> String {
        "hash".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
}
