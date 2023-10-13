use std::collections::HashMap;

#[derive(Default)]
struct Index {
    changes: HashMap<String, String>, 
}

impl Index {
    fn new() -> Self {
        Index {
            changes: HashMap::new(),
        }
    }

    fn add_change(&mut self, path: &str, hash: &str) {
        self.changes.insert(path.to_string(), hash.to_string());
    }

    fn remove_change(&mut self, path: &str) {
        self.changes.remove(path);
    }

    fn get_changes(&self) -> HashMap<String, String>{
        self.changes.clone()
    }
}

