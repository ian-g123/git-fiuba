use std::collections::HashMap;
use std::f32::consts::E;
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use crate::commands::command_errors::CommandError;

use super::changes_types::ChangeTypes;

pub struct WorkingTreeStatus{
    path: PathBuf,
    //previous_status: carpeta objects de la base de datos?
    changes: HashMap<String, ChangeTypes>,
    current_paths: Vec<String> // --> paths actuales
}

impl WorkingTreeStatus{
    pub fn new(path: String)->Self{
        let changes:HashMap<String, ChangeTypes> = HashMap::new();

        WorkingTreeStatus { path: PathBuf::from(path) , changes: changes, current_paths: Vec::new()}
    }

    pub fn compare(&mut self)-> Result<(), CommandError>{
        /* let Ok(entries) = fs::read_dir(self.path.clone()) else{
            return Err(CommandError::FileNotFound(path));
        };
        for entry in entries {
            let Ok(entry) = entry else{
                return Err(CommandError::FileNotFound(entry));
            };
            let entry_path = entry.path();
            let entry_copy = entry_path.clone();
            let Some(entry_name) = entry_copy.to_str() else{
                return Err(CommandError::FileNotFound);
            };
            self.current_paths.push(entry_name.to_string());
            if entry_path.is_dir() {
                // (1) getHash (2) check if previous_status contains hash --> Yes: _ , 
                // No: Add change(tipo)
                self.compare()?;
            }
            
            self.compare_entry(entry_path, entry_name);
        } */
        Ok(())
    }

    fn compare_entry(&mut self, entry__path: PathBuf, entry_name:&str){
        // compare to DB
        
        //self.add_change(ChangeType::_, entry_name);
    }

    fn add_change(&mut self, change_type: ChangeTypes, path: &str){
        _ = self.changes.insert(path.to_string(), change_type)
    }

    /* 
    1)
    Revisar cada file del WT con la base de datos (cambios en el contenido), Added, etc
    2) Revisar si hay paths en la base de datos q no están en el WT --> Deleted

     */
}

// Funciones incompletas --> por eso no hay tests