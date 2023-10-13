use std::{path::PathBuf, collections::HashMap};

use super::{staging_area::Index, changes_types::ChangeTypes};

pub struct ChangesNotStaged{
    working_tree: PathBuf,
    staging_area: Index,
    changes: HashMap<String, ChangeTypes>
}

impl ChangesNotStaged{
    pub fn new(working_tree: PathBuf, index: Index)-> Self{
        let changes :HashMap<String, ChangeTypes> = HashMap::new();
        ChangesNotStaged { working_tree: working_tree, staging_area: index, changes: changes }
    }

    pub fn get_changes(&self)-> HashMap<String, ChangeTypes>{
        self.changes.clone()
    }

    pub fn run(&self){
        let staged_changes = self.staging_area.get_changes();
        for (path, hash) in staged_changes.iter() {
            
        }
    }

}