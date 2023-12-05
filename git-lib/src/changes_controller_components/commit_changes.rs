use std::collections::HashMap;

use crate::objects::git_object::GitObjectTrait;
use crate::{
    command_errors::CommandError, logger::Logger, objects::tree::Tree,
    objects_database::ObjectsDatabase,
};

use super::changes_types::ChangeType;

pub fn get_changes_insertions_and_deletions(
    staging_area_changes: HashMap<String, String>,
    db: &ObjectsDatabase,
    logger: &mut Logger,
    commit_tree: Option<Tree>,
    changes_to_be_commited: &HashMap<String, ChangeType>,
) -> Result<(usize, usize, usize), CommandError> {
    let commit_tree = &commit_tree;
    let mut insertions = 0;
    let mut deletions = 0;
    let mut files_changed = 0;
    for (path, change_type) in changes_to_be_commited.iter() {
        logger.log(&format!("Checking for insertions/deletions: {}", path));
        let hash = staging_area_changes.get(path);
        match change_type {
            ChangeType::Added | ChangeType::Renamed => {
                logger.log(&format!("added: {}", path));

                if let Some(hash) = hash {
                    insertions += count_insertions(hash, logger, db)?;
                    files_changed += 1;
                }
            }
            ChangeType::Deleted => {
                if let Some(mut tree) = commit_tree.to_owned() {
                    logger.log(&format!("deleted: {}", path));

                    deletions += count_deletions(path, logger, &mut tree)?;
                    files_changed += 1;
                }
            }
            ChangeType::Modified => {
                logger.log(&format!("modified: {}", path));

                if let (Some(hash), Some(mut tree)) = (hash, commit_tree.to_owned()) {
                    let (inserted, deleted) =
                        count_modifications(path, hash, logger, &mut tree, db)?;
                    files_changed += 1;
                    insertions += inserted;
                    deletions += deleted;
                }
            }
            _ => {}
        }
    }

    Ok((files_changed, insertions, deletions))
}

fn count_insertions(
    path: &str,
    logger: &mut Logger,
    db: &ObjectsDatabase,
) -> Result<usize, CommandError> {
    let lines = read_content(path, logger, db)?;
    Ok(lines.len())
}

fn count_deletions(
    path: &str,
    logger: &mut Logger,
    commit_tree: &mut Tree,
) -> Result<usize, CommandError> {
    let lines = read_blob_content(path, logger, commit_tree)?;
    Ok(lines.len())
}

fn count_modifications(
    path: &str,
    data_base_path: &str,
    logger: &mut Logger,
    commit_tree: &mut Tree,
    db: &ObjectsDatabase,
) -> Result<(usize, usize), CommandError> {
    let last_commit_content = read_blob_content(path, logger, commit_tree)?;
    let current_content = read_content(data_base_path, logger, db)?;
    Ok(compare_content(
        last_commit_content,
        current_content,
        logger,
    ))
}

fn read_content(
    hash: &str,
    logger: &mut Logger,
    db: &ObjectsDatabase,
) -> Result<Vec<String>, CommandError> {
    let mut object = db.read_object(hash, logger)?;
    let content = object.content(None)?;
    let content = String::from_utf8(content)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let content_vec: Vec<&str> = content.lines().collect();
    let content_vec: Vec<String> = content_vec.iter().map(|s| s.to_string()).collect();
    Ok(content_vec)
}

fn read_blob_content(
    path: &str,
    _logger: &mut Logger,
    commit_tree: &mut Tree,
) -> Result<Vec<String>, CommandError> {
    let Some(mut object) = commit_tree.get_object_from_path(path) else {
        return Err(CommandError::FileNameError);
    };
    let Some(blob) = object.as_mut_blob() else {
        return Err(CommandError::UnknownObjectType);
    };
    let content = blob.content(None)?;
    let content_str: String = format!("{}", String::from_utf8_lossy(&content));
    let content_vec: Vec<&str> = content_str.lines().collect();
    let content_vec: Vec<String> = content_vec.iter().map(|s| s.to_string()).collect();
    Ok(content_vec)
}

fn compare_content(file1: Vec<String>, file2: Vec<String>, _logger: &mut Logger) -> (usize, usize) {
    let mut insertions: usize = 0;
    let mut deletions: usize = 0;

    let mut file1_read: Vec<String> = Vec::new();
    let mut file2_read: Vec<String> = Vec::new();
    let mut index1 = 0;
    let mut index2 = 0;

    loop {
        let line1 = get_element(&file1, index1);
        let line2 = get_element(&file2, index2);
        match (line1, line2) {
            (None, None) => {
                break;
            }
            (Some(line1), Some(line2)) => {
                if let Some(index) = file2_read.iter().position(|line| line == &line1) {
                    _ = file2_read.drain(..index + 1);
                    file2_read.push(line2.to_string());
                    insertions += index;
                } else if let Some(index) = file1_read.iter().position(|line| line == &line2) {
                    _ = file1_read.drain(..index + 1);
                    file1_read.push(line1.to_string());

                    deletions += index;
                } else if line1 == line2 {
                    insertions += file2_read.len();
                    deletions += file1_read.len();
                    file1_read = Vec::new();
                    file2_read = Vec::new();
                } else {
                    file1_read.push(line1.to_string());
                    file2_read.push(line2.to_string());
                }
            }
            (Some(line1), None) => {
                if let Some(index) = file2_read.iter().position(|line| line == &line1) {
                    _ = file2_read.drain(..index + 1);
                    insertions += index;
                } else {
                    file1_read.push(line1.to_string());
                }
            }

            (None, Some(line2)) => {
                if let Some(index) = file1_read.iter().position(|line| line == &line2) {
                    _ = file1_read.drain(..index + 1);
                    deletions += index;
                } else {
                    file2_read.push(line2.to_string());
                }
            }
        }

        index1 += 1;
        index2 += 1;
    }

    insertions += file2_read.len();
    deletions += file1_read.len();

    (insertions, deletions)
}

fn get_element(vector: &Vec<String>, index: usize) -> Option<String> {
    if index >= vector.len() {
        None
    } else {
        Some(vector[index].to_string())
    }
}
