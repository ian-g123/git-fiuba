use std::{collections::HashMap, fs::{File, self}, io::Read, path::{Path, PathBuf}, env::current_dir};

/* 

-Diferencias entre Index y el último commit, que es lo que está en la Base de datos

-Diferencias entre Index y Working Tree

-Archivos nuevos en el Working Tree que no están en el Staging Area ni en la Base de Datos

*/

use crate::commands::{command_errors::CommandError, hash_object_components::hash_object::HashObject, file_compressor::extract};

use super::{changes_types::ChangeType, change_object::ChangeObject};

pub struct Changes{
    /* working_tree: String,
    staging_area: HashMap<String, String>,
    data_base: String, */
    changes: HashMap<String, ChangeObject>
}

impl Changes{
    pub fn new()-> Self{
        let changes :HashMap<String, ChangeObject> = HashMap::new();
        /* let data_base = format!("{}/.git", working_tree); */
        Changes { changes: changes }
    }

    pub fn get_changes(&self)-> HashMap<String, ChangeObject>{
        self.changes.clone()
    }

    pub fn run(&mut self)-> Result<(), CommandError>{
        self.run_changes_not_staged()?;
        // falta: changes_staged, untracked
        Ok(())
    }

    fn run_changes_not_staged(&mut self)-> Result<(), CommandError>{
        /* let staged_changes = self.staging_area.get_changes();
        for change in staged_changes.iter() {
            let content_staged = change.get_content()?;
            let path = change.get_path();
            if let Some(file) = self.compare_file_name(path.clone()){
                //self.compare_content(content_staged, file, path);
            } 
        } */
        Ok(())
    }

    /* 
    M    work tree changed since index
    T    type changed in work tree since index
    D    deleted in work tree
	R    renamed in work tree
	C    copied in work tree
     */

    fn compare_file_name(&mut self, path: String) -> Option<File>{
        let Ok(file) = File::open(path.clone()) else{
            //_ = self.changes.insert(path, ChangeType::Deleted); // falta: check Renamed
            return None;
        };
        Some(file)
    }

    /* fn compare_content(&self, content_staged: Vec<u8>, mut file: File, path: String)->Result<(), CommandError>{
        let mut content_working_tree: Vec<u8> = Vec::new();
        if file.read_to_end(&mut content_working_tree).is_err(){
             
            return Err(CommandError::FileReadError(())) 
        }
        usar sha1. si diferen, cambio el contenido
    } */

    /// Compara las carpetas y archivos del Working Tree y el Staging Area. (falta refactor)
    fn compare(
        path_name: String,
        index: &HashMap<String, String>,
        changes: &mut HashMap<String, ChangeObject>,
        parent: &Option<String>,
    ) -> Result<(), CommandError> {
        let path = Path::new(&path_name);

        let Ok(entries) = fs::read_dir(path.clone()) else {
            return Err(CommandError::DirNotFound(path_name));
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return Err(CommandError::DirNotFound(path_name)); //cambiar!
            };
            let entry_path = entry.path();
            let entry_name = get_path_name(entry_path.clone())?;

            if entry_path.is_dir() {
                //let mut channges = HashMap::<String, ChangeObject>::new();
                Self::compare(entry_name.clone(), index, changes, parent)?;
                /* if !index.is_empty() {
                    _ = channges.insert(entry_name, Box::new(tree));
                    return Ok(());
                } */
                return Ok(());
            } else {
                /* let result = Self::compare_entry(&path_name, index, parent)?;
                if let Some(blob) = result {
                    _ = channges.insert(blob.get_hash(), Box::new(blob));
                } */
            }
        }
        Ok(())

    }

    fn compare_entry(path:String, index: HashMap<String, String>, parent_hash: String, current_hash:String)-> Result<ChangeObject, CommandError>{
        let (is_in_last_commit, name) = Self::is_in_last_commit(parent_hash, current_hash.clone())?;
        let change_type_working_tree : Option<ChangeType>;
        let change_type_staging_area : Option<ChangeType>;

        let change:ChangeObject;
        if is_in_last_commit{
            let current_name = get_path_name(PathBuf::from(path.clone()))?;
            if Self::check_is_renamed(name, current_name){
                if index.contains_key(&path){
                    change_type_working_tree = None;
                    change_type_staging_area = Some(ChangeType::Renamed);
                }else{
                    change_type_staging_area = None;
                    change_type_working_tree = Some(ChangeType::Renamed);
                }
            }else{
                change_type_working_tree = Some(ChangeType::Unmodified);
                change_type_staging_area = Some(ChangeType::Unmodified);
            }
        }else if index.contains_key(&path){ //falta: check_is_modified
            change_type_working_tree = None;
            change_type_staging_area = Some(ChangeType::Added);
        }else{
            change_type_working_tree = Some(ChangeType::Untracked);
            change_type_staging_area = Some(ChangeType::Untracked);
        }
        change = ChangeObject::new(current_hash, change_type_working_tree, change_type_staging_area);
        Ok(change)
    }

    

    fn is_in_last_commit(parent_hash: String, blob_hash: String)-> Result<(bool, String), CommandError>{
        let path = format!(
            ".git/objects/{}/{}",
            parent_hash[..2].to_string(),
            parent_hash[2..].to_string()
        );
        let data = Self::read_content(parent_hash)?;
        let data = extract(&data)?;
        let buf = String::from_utf8_lossy(&data).to_string();
        let lines: Vec<&str> = buf.split_terminator("\n").collect();
        for line in lines {
            let info: Vec<&str> = line.split_terminator(" ").collect();
            let hash_and_name:Vec<&str> = info[2].split("  ").collect();
            let (obj_type, obj_hash, name) = (info[1], hash_and_name[0], hash_and_name[1]);
            if obj_hash == blob_hash {
                return Ok((true, name.to_string()));
            }
            if obj_type == "tree" {
                return Self::is_in_last_commit(obj_hash.to_string(), blob_hash);
            }
        }
        Ok((false, String::new()))

    }

    /// Busca el contenido de un archivo en la Base de Datos y lo devuelve. Si no existe, devuelve error.
    fn read_content(hash: String) -> Result<Vec<u8>, CommandError> {
        let mut data: Vec<u8> = Vec::new();
        let path = format!(
            ".git/objects/{}/{}",
            hash[..2].to_string(),
            hash[2..].to_string()
        );
        let Ok(mut tree_file) = File::open(&path) else {
            return Err(CommandError::FileNotFound(path));
        };
        if tree_file.read_to_end(&mut data).is_err() {
            return Err(CommandError::FileReadError(path));
        }
        Ok(data)
    }

    fn check_is_renamed(name_data_base: String, current_name: String)->bool{
        name_data_base == current_name
    }

} 

/// Obtiene el directorio actual.
fn get_current_dir() -> Result<PathBuf, CommandError> {
    let Ok(current_dir) = current_dir() else {
        return Err(CommandError::DirNotFound("Current dir".to_string())); //cambiar
    };
    Ok(current_dir)
}    

/// Devuelve el nombre de un archivo o directorio dado un PathBuf.
fn get_path_name(path: PathBuf) -> Result<String, CommandError> {
    let Some(path_name) = path.to_str() else {
        return Err(CommandError::DirNotFound("".to_string())); //cambiar
    };
    Ok(path_name.to_string())
}

/// Devuelve el hash del path pasado. Si no existe, devuelve Error.
pub fn get_sha1(path: String, object_type: String, write: bool) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, write, false);
    let (hash, _) = hash_object.run_for_content(content)?;
    Ok(hash)
}

/// Lee el contenido de un archivo y lo devuelve. Si la operación falla, devuelve error.
pub fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

