use std::{collections::HashMap, fs::{File, self}, io::Read};

/* 

-Diferencias entre Index y el último commit, que es lo que está en la Base de datos

-Diferencias entre Index y Working Tree

-Archivos nuevos en el Working Tree que no están en el Staging Area ni en la Base de Datos

*/

/*
use crate::commands::command_errors::CommandError;

use super::{changes_types::ChangeType};

pub struct Changes{
    working_tree: String,
    staging_area: StagingArea,
    data_base: String,
    changes: HashMap<String, ChangeType>
}

impl Changes{
    pub fn new(working_tree: String, StagingArea: StagingArea)-> Self{
        let changes :HashMap<String, ChangeType> = HashMap::new();
        let data_base = format!("{}/.git", working_tree);
        Changes { working_tree: working_tree, staging_area: StagingArea, data_base: data_base, changes: changes }
    }

    pub fn get_changes(&self)-> HashMap<String, ChangeType>{
        self.changes.clone()
    }

    pub fn run(&mut self)-> Result<(), CommandError>{
        self.run_changes_not_staged()?;
        // falta: changes_staged, untracked
    }

    fn run_changes_not_staged(&mut self)-> Result<(), CommandError>{
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
            _ = self.changes.insert(path, ChangeType::Deleted); // falta: check Renamed
            return None;
        };
        Some(file)
    }

    fn compare_content(&self, content_staged: Vec<u8>, mut file: File, path: String)->Result<(), CommandError>{
        let mut content_working_tree: Vec<u8> = Vec::new();
        if file.read_to_end(&mut content_working_tree).is_err(){
             
            return Err(CommandError::FileReadError(())) 
        }
        usar sha1. si diferen, cambio el contenido
    }

} */

    

