use std::collections::HashMap;

use super::change::Change;

#[derive(Default)]
pub struct Index {
    changes: HashMap<String, Change>, 
}

impl Index {
    pub fn new() -> Self {
        Index {
            changes: HashMap::new(),
        }
    }

    pub fn add_change(&mut self, path: &str, change: Change) {
        self.changes.insert(path.to_string(), change);
    }

    pub fn remove_change(&mut self, path: &str) {
        self.changes.remove(path);
    }

    pub fn get_changes(&self) -> Vec<Change>{
        self.changes.values().cloned().collect()
    }
}

