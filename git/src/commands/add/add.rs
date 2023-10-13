use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;

/// Se obtiene el nombre de los archivos y directorios del workspace
/// Útil para cuando tengamos que hacer la interfaz de Stage Area
fn get_file_system_names() -> (HashSet<String>, HashSet<String>) { // TODO: remover los unwrap
    let mut files_names = HashSet::new();
    let mut directory_names = HashSet::new();

    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();

        let file_name = entry.file_name().to_str().unwrap().to_string();
        let directory_name = entry.path().to_str().unwrap().to_string();

        files_names.insert(file_name);
        directory_names.insert(directory_name);
    }
    (files_names, directory_names)
}

pub struct StagingArea {
    stagin_area: WorkingTree,
    pointers: HashMap<String, String>,
}

pub struct WorkingTree {
    file_system: HashMap<String, SavedObjects>,
}

enum SavedObjects {
    Blob(String),
    Tree(WorkingTree),
}

impl StagingArea {
    pub fn new() -> Self {
        StagingArea {
            stagin_area: WorkingTree::new(),
            pointers: HashMap::new(),
        }
    }

    pub fn add(&mut self, file_name: String) {
        self.stagin_area.add_file(file_name);
    }
}

impl WorkingTree {
    pub fn new() -> Self {
        WorkingTree {
            file_system: HashMap::new(),
        }
    }
    
    pub fn add_file(&mut self, file_name: String) {
        let file = SavedObjects::Blob(file_name.clone());
        self.file_system.insert(file_name, file);
    }

    pub fn add_tree(&mut self, folder_name: String) {
        let tree = SavedObjects::Tree(WorkingTree::new());
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
