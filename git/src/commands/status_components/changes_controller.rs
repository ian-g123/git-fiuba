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

use crate::{
    commands::{
        command_errors::CommandError,
        file_compressor::extract,
        hash_object_components::hash_object::HashObject,
        objects::{
            aux::get_name, git_object::GitObject, last_commit::build_last_commit_tree, tree::Tree,
        },
        staging_area::{self, StagingArea},
    },
    logger::Logger,
};

use super::{
    change_object::ChangeObject,
    changes_types::ChangeType,
    working_tree::{self, build_working_tree},
};

pub struct ChangesController {
    index_changes: HashMap<String, ChangeType>,
    working_tree_changes: HashMap<String, ChangeType>,
    untracked: Vec<String>,
}

impl ChangesController {
    /* fn print_tree(tree: Tree, logger: &mut Logger) {
        for (name, obj) in tree.get_objects().iter() {
            if let Some(tree2) = obj.as_tree() {
                logger.log(&format!("Found  tree: {}", name));
                Self::print_tree(tree2, logger);
            } else {
                logger.log(&format!("Found  blob: {}", name));
            }
        }
    } */
    pub fn new(logger: &mut Logger) -> Result<ChangesController, CommandError> {
        let commit_tree = build_last_commit_tree(logger)?;
        /* logger.log(&format!("Printing tree..."));
        if let Some(tree) = commit_tree {
            Self::print_tree(tree, logger);
        } */
        let index = StagingArea::open()?;
        let working_tree = build_working_tree()?;
        let (working_tree_changes, untracked) =
            Self::check_working_tree_status(working_tree, &index, &commit_tree, logger)?;
        let index_changes = Self::check_staging_area_status(&index, &commit_tree, logger)?;

        Ok(Self {
            index_changes,
            working_tree_changes,
            untracked,
        })
    }

    pub fn get_changes_to_be_commited(&self) -> &HashMap<String, ChangeType> {
        &self.index_changes
    }

    pub fn get_changes_not_staged(&self) -> &HashMap<String, ChangeType> {
        &self.working_tree_changes
    }

    pub fn get_untracked_files(&self) -> &Vec<String> {
        &self.untracked
    }

    fn check_staging_area_status(
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<HashMap<String, ChangeType>, CommandError> {
        let staging_files = staging_area.get_files();
        let Some(mut tree) = last_commit.to_owned() else {
            let changes: HashMap<String, ChangeType> = staging_files
                .iter()
                .map(|(path, _)| (path.to_string(), ChangeType::Added)) // Aquí puedes aplicar una función de transformación
                .collect();
            return Ok(changes);
        };

        let mut changes: HashMap<String, ChangeType> = HashMap::new();
        for (path, hash) in staging_files.iter() {
            let has_path = tree.has_blob_from_path(path);
            logger.log(&format!("Path: {}, {}", path, has_path));
            let (has_hash, name) = tree.has_blob_from_hash(hash, logger)?;
            logger.log(&format!("Hash found: {}", has_hash));

            let actual_name = get_name(path)?;
            if !has_path && !has_hash {
                _ = changes.insert(path.to_string(), ChangeType::Added);
            } else if has_path && !has_hash {
                _ = changes.insert(path.to_string(), ChangeType::Modified);
            } else if has_path && has_hash {
                _ = changes.insert(path.to_string(), ChangeType::Unmodified);
            } else if has_hash && name != actual_name {
                _ = changes.insert(path.to_string(), ChangeType::Renamed);
            }
        }
        Self::get_deleted_changes_index(staging_area, &mut changes)?;
        Ok(changes)
    }

    fn check_working_tree_status(
        mut working_tree: Tree,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<(HashMap<String, ChangeType>, Vec<String>), CommandError> {
        let mut wt_changes: HashMap<String, ChangeType> = HashMap::new();
        let mut untracked: Vec<String> = Vec::new();

        Self::check_working_tree_aux(
            &mut working_tree,
            staging_area,
            last_commit,
            &mut wt_changes,
            &mut untracked,
            logger,
        )?;
        Self::get_deleted_changes_working_tree(&mut working_tree, staging_area, &mut wt_changes);
        Ok((wt_changes, untracked))
    }

    fn get_deleted_changes_working_tree(
        working_tree: &mut Tree,
        staging_area: &StagingArea,
        changes: &mut HashMap<String, ChangeType>,
    ) {
        let staged_files: HashMap<String, String> = staging_area.get_files();
        let files: Vec<&String> = staged_files.keys().collect();
        let deleted_changes = working_tree.get_deleted_blobs_from_path(files);
        for deleted_file in deleted_changes.iter() {
            _ = changes.insert(deleted_file.to_string(), ChangeType::Deleted)
        }
    }

    fn get_deleted_changes_index(
        staging_area: &StagingArea,
        changes: &mut HashMap<String, ChangeType>,
    ) -> Result<(), CommandError> {
        let deleted_changes = staging_area.get_deleted_files()?;
        for deleted_file in deleted_changes.iter() {
            _ = changes.insert(deleted_file.to_string(), ChangeType::Deleted)
        }
        Ok(())
    }

    fn check_working_tree_aux(
        tree: &mut Tree,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        changes: &mut HashMap<String, ChangeType>,
        untracked: &mut Vec<String>,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        for (_, object) in tree.get_objects().iter_mut() {
            if let Some(mut new_tree) = object.as_tree() {
                Self::check_working_tree_aux(
                    &mut new_tree,
                    staging_area,
                    last_commit,
                    changes,
                    untracked,
                    logger,
                )?
            } else {
                Self::check_file_status(
                    object,
                    staging_area,
                    last_commit,
                    changes,
                    untracked,
                    logger,
                )?;
            }
        }
        Ok(())
    }

    fn check_file_status(
        object: &mut GitObject,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        changes: &mut HashMap<String, ChangeType>,
        untracked: &mut Vec<String>,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let Some(path) = object.get_path() else {
            return Err(CommandError::FileNameError);
        };

        let hash = object.get_hash_string()?;
        let has_path = staging_area.has_file_from_path(&path);
        let has_path_renamed = staging_area.has_file_renamed(&path, &hash);
        let has_hash = staging_area.has_file_from_hash(&hash);
        //let actual_name = get_name(&path)?;
        let isnt_in_last_commit = {
            if let Some(mut tree) = last_commit.to_owned() {
                let (is_in_last_commit, _) = tree.has_blob_from_hash(&hash, logger)?;
                !is_in_last_commit && !tree.has_blob_from_path(&path)
            } else {
                true
            }
        };
        if !has_path && !has_hash && isnt_in_last_commit {
            untracked.push(path);
        } else if has_path && !has_hash {
            _ = changes.insert(path, ChangeType::Modified);
        } else if has_path && has_hash {
            _ = changes.insert(path, ChangeType::Unmodified);
        } else if has_path_renamed {
            _ = changes.insert(path, ChangeType::Renamed);
        }
        Ok(())
    }
}
/*
falta: copy, type changed, dif entre modified y added, deleted del WT
*/

/*     /* pub fn get_changes(&self) -> HashMap<String, ChangeObject> {
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
 */
