use crate::{
    command_errors::CommandError,
    logger::Logger,
    objects::{git_object::GitObject, tree::Tree},
    objects_database::ObjectsDatabase,
    staging_area::StagingArea,
    utils::aux::get_name,
};
use std::{collections::HashMap, fs::File, io::Read};

use super::{changes_types::ChangeType, working_tree::build_working_tree};

/// Contiene información acerca de:
/// - Diferencias entre Index y el último commit, que es lo que está en la Base de datos
/// - Diferencias entre Index y Working Tree
/// - Archivos nuevos en el Working Tree que no están en el Staging Area ni en la Base de Datos
pub struct ChangesController {
    index_changes: HashMap<String, ChangeType>,
    working_tree_changes: HashMap<String, ChangeType>,
    untracked: Vec<String>,
    untracked_files: Vec<String>,
    unmerged_changes: HashMap<String, ChangeType>,
}

impl ChangesController {
    /// Crea un nuevo ChangesController que contiene información acerca de todos los tipos de cambios
    /// del index y el working tree desde el último commit.
    pub fn new(
        db: &ObjectsDatabase,
        git_path: &str,
        working_dir: &str,
        logger: &mut Logger,
        commit_tree: Option<Tree>,
    ) -> Result<ChangesController, CommandError> {
        logger.log(&format!(
            "ChangesController::new\ngit_path: {}\nworking_dir: {}",
            git_path, working_dir
        ));
        let index = StagingArea::open(git_path)?;
        logger.log("stagin area opened");
        let working_tree = build_working_tree(working_dir)?;
        logger.log("build_working_tree success");
        let index_changes = Self::check_staging_area_status(db, &index, &commit_tree, logger)?;
        let (working_tree_changes, untracked, untracked_files) = Self::check_working_tree_status(
            working_dir,
            working_tree,
            &index,
            &commit_tree,
            logger,
            &index_changes,
        )?;
        let unmerged_changes = Self::check_unmerged_paths(&index, logger)?;
        Ok(Self {
            index_changes,
            working_tree_changes,
            untracked,
            untracked_files,
            unmerged_changes,
        })
    }

    /// Devuelve los cambios que se incluirán en el próximo commit.
    pub fn get_changes_to_be_commited(&self) -> &HashMap<String, ChangeType> {
        &self.index_changes
    }

    /// Devuelve los cambios que no se incluirán en el próximo commit.
    pub fn get_changes_not_staged(&self) -> &HashMap<String, ChangeType> {
        &self.working_tree_changes
    }

    /// Devuelve los archivos desconocidos para git.
    pub fn get_untracked_files(&self) -> &Vec<String> {
        &self.untracked
    }

    /// Devuelve los cambios que no están mergeados.
    pub fn get_unmerged_changes(&self) -> &HashMap<String, ChangeType> {
        &self.unmerged_changes
    }

    pub fn get_untracked_files_interface(&self) -> &Vec<String> {
        &self.untracked_files
    }

    /// Recolecta información sobre los merge conflicts
    fn check_unmerged_paths(
        staging_area: &StagingArea,
        logger: &mut Logger,
    ) -> Result<HashMap<String, ChangeType>, CommandError> {
        let mut changes: HashMap<String, ChangeType> = HashMap::new();
        let unmerged_files = staging_area.get_unmerged_files();
        for (path, (common, head, destin)) in unmerged_files.iter() {
            let path_str = path.to_string();
            match (common, head, destin) {
                (Some(_), Some(_), Some(_)) => {
                    _ = changes.insert(path_str, ChangeType::ModifiedByBoth);
                }
                (None, Some(_), Some(_)) => {
                    _ = changes.insert(path_str, ChangeType::AddedByBoth);
                }
                (Some(_), Some(_), None) => {
                    _ = changes.insert(path_str, ChangeType::DeletedByThen);
                }
                (Some(_), None, Some(_)) => {
                    _ = changes.insert(path_str, ChangeType::DeletedByUs);
                }
                _ => {
                    return Err(CommandError::MergeConflict(
                        "Invalid merge conflict".to_string(),
                    ))
                }
            }
        }
        Ok(changes)
    }

    /// Obtiene los cambios que se incluirán en el próximo commit.
    fn check_staging_area_status(
        db: &ObjectsDatabase,
        staging_area: &StagingArea,
        last_commit_tree: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<HashMap<String, ChangeType>, CommandError> {
        let staging_files = staging_area.get_files();
        let Some(mut tree) = last_commit_tree.to_owned() else {
            let changes: HashMap<String, ChangeType> = staging_files
                .iter()
                .map(|(path, _)| (path.to_string(), ChangeType::Added))
                .collect();
            return Ok(changes);
        };
        let mut changes: HashMap<String, ChangeType> =
            Self::check_files_in_staging_area(staging_files, logger, &mut tree)?;
        Self::get_deleted_changes_index(db, last_commit_tree, staging_area, &mut changes)?;
        logger.log(&format!(
            "Staging area files: {:?}",
            staging_area.get_files().keys()
        ));

        logger.log(&format!("Staging area changes: {:?}", changes.keys()));

        Ok(changes)
    }

    /// Revisa los archivos guardados en el index para obtener el tipo de cambio que sufrieron.
    fn check_files_in_staging_area(
        staging_files: HashMap<String, String>,
        logger: &mut Logger,
        tree: &mut Tree,
    ) -> Result<HashMap<String, ChangeType>, CommandError> {
        let mut changes: HashMap<String, ChangeType> = HashMap::new();
        for (path, hash) in staging_files.iter() {
            let has_path = tree.has_blob_from_path(path, logger);
            let (has_hash, name) = tree.has_blob_from_hash(hash, logger)?;
            logger.log(&format!(
                "Path: {}, has_name: {}, has_hash: {}",
                path, has_path, has_hash
            ));
            let actual_name = get_name(path)?;
            if !has_path && !has_hash {
                logger.log(&format!("{} was added", path));
                _ = changes.insert(path.to_string(), ChangeType::Added);
            } else if has_path && !has_hash {
                logger.log(&format!("{} was modified", path));
                _ = changes.insert(path.to_string(), ChangeType::Modified);
            } else {
                _ = changes.insert(path.to_string(), ChangeType::Unmodified);
            }
        }
        Ok(changes)
    }

    /// Obtiene los cambios del working tree respecto al staging area.
    fn check_working_tree_status(
        working_dir: &str,
        mut working_tree: Tree,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        logger: &mut Logger,
        staged_changes: &HashMap<String, ChangeType>,
    ) -> Result<(HashMap<String, ChangeType>, Vec<String>, Vec<String>), CommandError> {
        let mut wt_changes: HashMap<String, ChangeType> = HashMap::new();
        let mut untracked: Vec<String> = Vec::new();
        let mut untracked_files: Vec<String> = Vec::new();
        Self::check_working_tree_aux(
            working_dir,
            &mut working_tree,
            staging_area,
            last_commit,
            &mut wt_changes,
            &mut untracked,
            &mut untracked_files,
            logger,
            staged_changes,
        )?;
        Self::get_deleted_changes_working_tree(
            &mut working_tree,
            staging_area,
            &mut wt_changes,
            logger,
        );
        logger.log(&format!("Changes not staged: {:?}", wt_changes.keys()));
        logger.log(&format!("untracked files: {:?}", untracked));

        Ok((wt_changes, untracked, untracked_files))
    }

    /// Obtiene los archivos eliminados en el working tree, pero presentes en el index.
    fn get_deleted_changes_working_tree(
        working_tree: &mut Tree,
        staging_area: &StagingArea,
        changes: &mut HashMap<String, ChangeType>,
        logger: &mut Logger,
    ) {
        let staged_files: HashMap<String, String> = staging_area.get_files();
        let files: Vec<&String> = staged_files.keys().collect();
        let files: Vec<&str> = files.iter().map(|s| s.as_str()).collect();

        let deleted_changes = working_tree.get_deleted_blobs_from_path(files, logger);
        for deleted_file in deleted_changes.iter() {
            _ = changes.insert(deleted_file.to_string(), ChangeType::Deleted)
        }
    }

    /// Obtiene los archivos eliminados en el index, pero presentes en el último commit.
    fn get_deleted_changes_index(
        db: &ObjectsDatabase,
        last_commit_tree: &Option<Tree>,
        staging_area: &StagingArea,
        changes: &mut HashMap<String, ChangeType>,
    ) -> Result<(), CommandError> {
        let deleted_changes = staging_area.get_deleted_files(last_commit_tree);
        for deleted_file in deleted_changes.iter() {
            _ = changes.insert(deleted_file.to_string(), ChangeType::Deleted)
        }
        Ok(())
    }

    /// Obtiene los cambios del working tree respecto al staging area.
    fn check_working_tree_aux(
        working_dir: &str,
        tree: &mut Tree,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        changes: &mut HashMap<String, ChangeType>,
        untracked: &mut Vec<String>,
        untracked_files: &mut Vec<String>,
        logger: &mut Logger,
        staged_changes: &HashMap<String, ChangeType>,
    ) -> Result<(), CommandError> {
        let mut untracked_number = 0;
        let mut total_files_dir = 0;
        for (_, (object_hash, object_opt)) in tree.get_objects().iter_mut() {
            let mut object = object_opt.to_owned().ok_or(CommandError::ShallowTree)?;
            if let Some(mut new_tree) = object.as_tree() {
                Self::check_working_tree_aux(
                    working_dir,
                    &mut new_tree,
                    staging_area,
                    last_commit,
                    changes,
                    untracked,
                    untracked_files,
                    logger,
                    staged_changes,
                )?
            } else {
                let (is_untracked, path) = Self::check_file_status(
                    working_dir,
                    &mut object,
                    staging_area,
                    last_commit,
                    changes,
                    untracked,
                    logger,
                )?;

                total_files_dir += 1;
                if is_untracked {
                    untracked_files.push(path);
                    untracked_number += 1;
                }
            }
        }
        if untracked_number == total_files_dir {
            Self::set_untracked_folder(untracked, untracked_number, logger, staged_changes)
        }
        Ok(())
    }

    /// Si todos los archivos de una carpeta son untracked, nos quedamos con el path del directorio.
    fn set_untracked_folder(
        untracked: &mut Vec<String>,
        untracked_number: usize,
        _: &mut Logger,
        staged_changes: &HashMap<String, ChangeType>,
    ) {
        if untracked.len() == 0 || untracked_number == 0 {
            return;
        }
        let last_file = &untracked[untracked.len() - 1];
        let file_parts: Vec<&str> = last_file.split('/').collect();
        if file_parts.len() == 1 {
            return;
        }
        let dir = format!("{}/", file_parts[..file_parts.len() - 1].join("/"));
        if staged_changes
            .iter()
            .any(|(string, _)| string.contains(&dir))
        {
            return;
        }
        let original_size = untracked.len();
        let new_size = original_size - untracked_number;
        untracked.retain(|file| !file.starts_with(&dir));

        untracked.truncate(new_size);
        untracked.insert(0, dir);
    }

    /// Obtiene el tipo de cambio del archivo respecto al staging area.
    fn check_file_status(
        working_dir: &str,
        object: &mut GitObject,
        staging_area: &StagingArea,
        last_commit: &Option<Tree>,
        changes: &mut HashMap<String, ChangeType>,
        untracked: &mut Vec<String>,
        logger: &mut Logger,
    ) -> Result<(bool, String), CommandError> {
        let Some(path) = object.get_path() else {
            return Err(CommandError::ObjectPathError);
        };

        let hash = object.get_hash_string()?;
        let has_path = staging_area.has_file_from_path(&path);
        let has_path_renamed = staging_area.has_file_renamed(&path, &hash);
        let has_hash = staging_area.has_file_from_hash(&hash);

        let isnt_in_last_commit = check_isnt_in_last_commit(last_commit, &path, &hash, logger)?;
        if !has_path
            && (!has_hash || (has_hash && content_differs(&path, object)?))
            && isnt_in_last_commit
        {
            logger.log(&format!("{} is untracked", path));

            untracked.push(path.clone());
            return Ok((true, path));
        } else if has_path && !has_hash {
            logger.log(&format!("{} was modified", path));

            _ = changes.insert(path.clone(), ChangeType::Modified);
        } else {
            _ = changes.insert(path.clone(), ChangeType::Unmodified);
        }
        Ok((false, path))
    }
}

/// Devuelve true si el contenido del objeto y el path pasados difieren.
fn content_differs(path: &str, object: &mut GitObject) -> Result<bool, CommandError> {
    let staged_content: String = String::from_utf8(object.content(None)?)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;

    let Ok(mut current_file) = File::open(path) else {
        return Err(CommandError::FileOpenError(path.to_string()));
    };
    let mut current_content = String::new();
    current_file
        .read_to_string(&mut current_content)
        .map_err(|_: std::io::Error| CommandError::FileReadError(path.to_string()))?;

    Ok(current_content == staged_content)
}

fn check_isnt_in_last_commit(
    last_commit: &Option<Tree>,
    path: &str,
    hash: &str,
    logger: &mut Logger,
) -> Result<bool, CommandError> {
    if let Some(mut tree) = last_commit.to_owned() {
        let (is_in_last_commit, name) = tree.has_blob_from_hash(hash, logger)?;
        return Ok((!is_in_last_commit || name != get_name(path)?)
            && !tree.has_blob_from_path(path, logger));
    }
    Ok(true)
}
