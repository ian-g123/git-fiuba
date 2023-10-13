use std::collections::HashMap;

#[derive(Default)]
pub struct Index {
    changes: HashMap<String, String>, 
}

impl Index {
    pub fn new() -> Self {
        Index {
            changes: HashMap::new(),
        }
    }

    pub fn add_change(&mut self, path: &str, hash: &str) {
        self.changes.insert(path.to_string(), hash.to_string());
    }

    pub fn remove_change(&mut self, path: &str) {
        self.changes.remove(path);
    }

    pub fn get_changes(&self) -> HashMap<String, String>{
        self.changes.clone()
    }
}

