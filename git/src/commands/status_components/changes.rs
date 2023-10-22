use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

/*

-Diferencias entre Index y el último commit, que es lo que está en la Base de datos

-Diferencias entre Index y Working Tree

-Archivos nuevos en el Working Tree que no están en el Staging Area ni en la Base de Datos

*/

use crate::commands::{
    command_errors::CommandError,
    file_compressor::extract,
    hash_object_components::hash_object::HashObject,
    objects::{aux::get_name, tree::Tree},
    staging_area::{self, StagingArea},
};

use super::{
    change_object::ChangeObject, changes_types::ChangeType, working_tree::build_working_tree,
};

pub struct Changes {
    index_changes: HashMap<String, ChangeObject>,
    working_tree_changes: HashMap<String, ChangeObject>,
}

impl Changes {
    pub fn new() -> Result<(), CommandError> {
        let mut changes: HashMap<String, ChangeObject> = HashMap::new();
        let commit_tree = get_commit_tree()?;
        let mut index = StagingArea::open()?;
        let working_tree = build_working_tree()?;
        /* let tree_staged = index.get_working_tree_staged(logger)
        let current_dir = get_current_dir()?;
        Self::compare(current_dir, index, &mut changes, &commit_tree)?;
        Self::get_deleted_changes(index, &mut changes); */
        //Ok(Changes { changes: changes })
        Ok(())
    }

    fn check_staging_area_status(
        staging_files: &HashMap<String, String>,
        last_commit: Tree,
    ) -> Result<(), CommandError> {
        for (path, hash) in staging_files.iter() {
            let has_path = last_commit.has_blob_from_path(path);
            let (has_hash, name) = last_commit.has_blob_from_hash(hash)?;
            let actual_name = get_name(path)?;
            if !has_path && !has_hash {
                // es added
            } else if has_path && !has_hash {
                // es modified
            } else if has_hash && name != actual_name {
                // es renamed
            } else {
                // unmodified
            }
        }
        //get_deleted_changes_staging_area()
        Ok(())
    }

    fn check_working_tree_status(
        mut working_tree: Tree,
        staging_area: &StagingArea,
        last_commit: Tree,
    ) -> Result<(), CommandError> {
        for (name, object) in working_tree.get_objects().iter_mut() {
            let Some(path) = object.get_name() else {
                return Err(CommandError::FileNameError);
            };
            let hash = object.get_hash_string()?;
            let has_path = staging_area.has_file_from_path(&path);
            let has_path_renamed = staging_area.has_file_renamed(&path, &hash);
            let has_hash = staging_area.has_file_from_hash(&hash);
            let actual_name = get_name(&path)?;
            let (is_in_last_commit, _) = last_commit.has_blob_from_hash(&hash)?;
            if !has_path
                && !has_hash
                && !last_commit.has_blob_from_path(&path)
                && !is_in_last_commit
            {
                // es untracked
            } else if has_path && !has_hash {
                // es modified
            } else if has_path_renamed {
                // es renamed
            } else {
                // unmodified
            }
        }
        //let staged_files: Vec<&String> = staging_area.get_files().keys().collect();
        //let deleted_changes = working_tree.get_deleted_blobs_from_path(staged_files);
        Ok(())
    }

    /*
    falta: copy, type changed, dif entre modified y added, deleted del WT
    */

    /* pub fn get_changes(&self) -> HashMap<String, ChangeObject> {
        self.changes.clone()
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
                Self::compare(entry_name.clone(), index, changes, parent)?;
                return Ok(());
            } else {
                /* let current_hash = get_sha1(entry_name.clone(), "blob".to_string())?;
                if let Some(parent_hash) = parent {
                    Self::compare_entry(&entry_name, index, parent_hash.to_string(), current_hash)?;
                } */
            }
        }
        Ok(())
    }

    fn compare_entry(
        path: &str,
        index: &HashMap<String, String>,
        parent_hash: String,
        current_hash: String,
    ) -> Result<ChangeObject, CommandError> {
        let (is_in_last_commit, name) = Self::is_in_last_commit(parent_hash, current_hash.clone())?;
        let change_type_working_tree: Option<ChangeType>;
        let change_type_staging_area: Option<ChangeType>;

        let change: ChangeObject;
        if is_in_last_commit {
            let current_name = get_path_name(PathBuf::from(path.clone()))?;
            if Self::check_is_renamed(name, current_name) {
                if index.contains_key(path) {
                    change_type_working_tree = None;
                    change_type_staging_area = Some(ChangeType::Renamed);
                } else {
                    change_type_staging_area = None;
                    change_type_working_tree = Some(ChangeType::Renamed);
                }
            } else {
                change_type_working_tree = Some(ChangeType::Unmodified);
                change_type_staging_area = Some(ChangeType::Unmodified);
            }
        } else if index.contains_key(path) {
            //falta: check_is_modified
            change_type_working_tree = None;
            change_type_staging_area = Some(ChangeType::Added);
        } else {
            change_type_working_tree = Some(ChangeType::Untracked);
            change_type_staging_area = Some(ChangeType::Untracked);
        }
        change = ChangeObject::new(
            current_hash,
            change_type_working_tree,
            change_type_staging_area,
        );
        Ok(change)
    }

    fn get_deleted_changes(
        index: &HashMap<String, String>,
        changes: &mut HashMap<String, ChangeObject>,
    ) {
        for (path, hash) in index.iter() {
            if !changes.contains_key(path) {
                let change = ChangeObject::new(hash.to_string(), None, Some(ChangeType::Deleted)); //deleted in WT
                _ = changes.insert(path.to_string(), change);
            }
        }
    }

    fn is_in_last_commit(
        parent_hash: String,
        blob_hash: String,
    ) -> Result<(bool, String), CommandError> {
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
            let hash_and_name: Vec<&str> = info[2].split("  ").collect();
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

    fn check_is_renamed(name_data_base: String, current_name: String) -> bool {
        name_data_base == current_name
    }
}

/// Obtiene el directorio actual.
fn get_current_dir() -> Result<String, CommandError> {
    let Ok(current_dir) = current_dir() else {
        return Err(CommandError::DirNotFound("Current dir".to_string())); //cambiar
    };
    let Some(current_dir_str) = current_dir.to_str() else {
        return Err(CommandError::DirNotFound("Current dir".to_string())); //cambiar
    };
    Ok(current_dir_str.to_string())
}

/// Devuelve el nombre de un archivo o directorio dado un PathBuf.
fn get_path_name(path: PathBuf) -> Result<String, CommandError> {
    let Some(path_name) = path.to_str() else {
        return Err(CommandError::DirNotFound("".to_string())); //cambiar
    };
    Ok(path_name.to_string())
}

/* /// Devuelve el hash del path pasado. Si no existe, devuelve Error.
pub fn get_sha1(path: String, object_type: String) -> Result<String, CommandError> {
    let content = read_file_contents(&path)?;
    let files = [path].to_vec();
    let hash_object = HashObject::new(object_type, files, false, false);
    let (hash, _) = hash_object.run_for_content(content)?;
    Ok(hash)
} */

/// Lee el contenido de un archivo y lo devuelve. Si la operación falla, devuelve error.
pub fn read_file_contents(path: &str) -> Result<Vec<u8>, CommandError> {
    let mut file = File::open(path).map_err(|_| CommandError::FileNotFound(path.to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| CommandError::FileReadError(path.to_string()))?;
    Ok(data)
}

fn get_current_branch() -> Result<String, CommandError> {
    let mut branch = String::new();
    let mut parent = String::new();
    let path = ".git/HEAD";
    let Ok(mut head) = File::open(path) else {
        return Err(CommandError::FileOpenError(path.to_string())); //Cmbiar: not git repository
    };

    if head.read_to_string(&mut branch).is_err() {
        return Err(CommandError::FileReadError(path.to_string()));
    }

    let branch = branch.trim();
    let Some(branch) = branch.split(" ").last() else {
        return Err(CommandError::FileReadError(path.to_string()));
    };
    Ok(branch.to_string())
}

/// Obtiene el hash del Commit padre. Si no tiene p
fn get_last_commit() -> Result<Option<String>, CommandError> {
    let mut parent = String::new();
    let branch = get_current_branch()?;
    let branch_path = format!(".git/{}", branch);
    let Ok(mut branch_file) = File::open(branch_path.clone()) else {
        return Ok(None);
    };

    if branch_file.read_to_string(&mut parent).is_err() {
        return Err(CommandError::FileReadError(branch_path.to_string()));
    }

    let parent = parent.trim();
    Ok(Some(parent.to_string()))
}

fn get_commit_tree() -> Result<Option<String>, CommandError> {
    let Some(last_commit) = get_last_commit()? else {
        return Ok(None);
    };
    let path = format!(
        "{}/{}",
        last_commit[0..2].to_string(),
        last_commit[2..].to_string()
    );
    let Ok(mut commit_file) = File::open(path.clone()) else {
        return Err(CommandError::FileReadError(path.to_string()));
    };
    let mut buf: String = String::new();
    if commit_file.read_to_string(&mut buf).is_err() {
        return Err(CommandError::FileReadError(path.to_string()));
    }
    let info: Vec<&str> = buf.split("\n").collect();
    let tree_info = info[0].to_string();
    let tree_info: Vec<&str> = tree_info.split(" ").collect();
    let tree_hash = tree_info[1];
    Ok(Some(tree_hash.to_string()))
}
