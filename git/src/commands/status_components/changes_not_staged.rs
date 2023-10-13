use std::{path::PathBuf, collections::HashMap, fs::{File, self}, io::Read};

use crate::{commands::command_errors::CommandError, index::staging_area::Index};

use super::{changes_types::ChangeTypes};

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

    pub fn run(&mut self)-> Result<(), CommandError>{
        let staged_changes = self.staging_area.get_changes();
        for change in staged_changes.iter() {
            let content_staged = change.get_content()?;
            let path = change.get_path();
            if let Some(file) = self.compare_file_name(path.clone()){
                self.compare_content(content_staged, file, path);
            } 
        }
        Ok(())
    }

    fn compare_file_name(&mut self, path: String) -> Option<File>{
        let Ok(file) = File::open(path.clone()) else{
            _ = self.changes.insert(path, ChangeTypes::Deleted); // falta: check Renamed
            return None;
        };
        Some(file)
    }

    fn compare_content(&self, content_staged: Vec<u8>, mut file: File, path: String)->Result<(), CommandError>{
        let mut content_working_tree: Vec<u8> = Vec::new();
        if file.read_to_end(&mut content_working_tree).is_err(){
            /* let file_name = 
            return Err(CommandError::FileReadError(())) */
        }
        Ok(())
    }

}

    

