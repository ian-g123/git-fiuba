use std::env::join_paths;
use std::fs::File;
use std::{collections::HashMap, fs, io::Read};

use crate::file_compressor::extract;
use crate::join_paths;
use crate::objects::git_object::{self, GitObjectTrait};
use crate::{
    command_errors::CommandError, logger::Logger, objects::tree::Tree,
    objects_database::ObjectsDatabase,
};

use super::{changes_controller::ChangesController, changes_types::ChangeType};

pub struct CommitChanges;

impl CommitChanges {
    pub fn new(
        staging_area_changes: HashMap<String, String>,
        db: &ObjectsDatabase,
        logger: &mut Logger,
        commit_tree: Option<Tree>,
        changes_to_be_commited: &HashMap<String, ChangeType>,
        curr_path: &str,
    ) -> Result<(usize, usize, usize), CommandError> {
        let (files_changed, insertions, deletions) = Self::get_changes_insertions_deletions(
            staging_area_changes,
            db,
            logger,
            &commit_tree,
            changes_to_be_commited,
            curr_path,
        )?;
        Ok((files_changed, insertions, deletions))
    }

    fn get_changes_insertions_deletions(
        staging_area_changes: HashMap<String, String>,
        db: &ObjectsDatabase,
        logger: &mut Logger,
        commit_tree: &Option<Tree>,
        changes_to_be_commited: &HashMap<String, ChangeType>,
        curr_path: &str,
    ) -> Result<(usize, usize, usize), CommandError> {
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
                        logger.log(&format!("Hash: {}", hash));

                        logger.log(&format!("Counting insertions"));

                        insertions += count_insertions(&hash, logger, db)?;
                        logger.log(&format!("insertions counted"));

                        files_changed += 1;
                    }
                }
                ChangeType::Deleted => {
                    if let Some(tree) = commit_tree {
                        logger.log(&format!("deleted: {}", path));

                        deletions += count_deletions(path, logger, &tree)?;
                        files_changed += 1;
                    }
                }
                ChangeType::Modified => {
                    logger.log(&format!("modified: {}", path));

                    if let (Some(hash), Some(tree)) = (hash, commit_tree) {
                        let (inserted, deleted) =
                            count_modifications(path, hash, logger, &tree, db)?;
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
    commit_tree: &Tree,
) -> Result<usize, CommandError> {
    let lines = read_blob_content(path, logger, commit_tree)?;
    Ok(lines.len())
}

fn count_modifications(
    path: &str,
    data_base_path: &str,
    logger: &mut Logger,
    commit_tree: &Tree,
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
    logger.log(&format!("content {:?}", content));
    let content_vec: Vec<&str> = content.lines().collect();
    logger.log(&format!("Vec {:?}", content_vec));
    let content_vec: Vec<String> = content_vec.iter().map(|s| s.to_string()).collect();
    Ok(content_vec)
}

fn read_blob_content(
    path: &str,
    logger: &mut Logger,
    commit_tree: &Tree,
) -> Result<Vec<String>, CommandError> {
    let Some(mut object) = commit_tree.get_blob(path, logger) else {
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

fn compare_content(file1: Vec<String>, file2: Vec<String>, logger: &mut Logger) -> (usize, usize) {
    let mut insertions: usize = 0;
    let mut deletions: usize = 0;

    let mut file1_read: Vec<String> = Vec::new();
    let mut file2_read: Vec<String> = Vec::new();
    let mut index1 = 0;
    let mut index2 = 0;

    loop {
        let line1 = get_element(&file1, index1);
        let line2 = get_element(&file2, index2);
        logger.log(&format!("f1: {:?}, f2: {:?}", file1_read, file2_read));
        match (line1, line2) {
            (None, None) => {
                break;
            }
            (Some(line1), Some(line2)) => {
                logger.log(&format!("Line1: {:?}, Line2: {:?}", line1, line2));

                if let Some(index) = file2_read.iter().position(|line| line == &line1) {
                    logger.log(&format!("Index1: {}", index));
                    _ = file2_read.drain(..index + 1);
                    file2_read.push(line2.to_string());
                    insertions += index;
                } else if let Some(index) = file1_read.iter().position(|line| line == &line2) {
                    logger.log(&format!("Index2: {}", index));

                    _ = file1_read.drain(..index + 1);
                    file1_read.push(line1.to_string());

                    deletions += index;
                } else if line1 == line2 {
                    insertions += file2_read.len();
                    deletions += file1_read.len();
                    file1_read = Vec::new();
                    file2_read = Vec::new();
                } else if line1 != line2
                    || (line1 == line2 && (!file1_read.is_empty() || !file2_read.is_empty()))
                {
                    file1_read.push(line1.to_string());
                    file2_read.push(line2.to_string());
                }
            }
            (Some(line1), None) => {
                if let Some(index) = file2_read.iter().position(|line| line == &line1) {
                    logger.log(&format!("Index3: {}", index));
                    _ = file2_read.drain(..index + 1);
                    insertions += index;
                } else {
                    file1_read.push(line1.to_string());
                }
            }

            (None, Some(line2)) => {
                if let Some(index) = file1_read.iter().position(|line| line == &line2) {
                    logger.log(&format!("Index4: {}", index));

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
/*

fn merge_difs(
    common_not_changed_in_head: HashMap<usize, String>,
    head_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
    common_not_changed_in_destin: HashMap<usize, String>,
    destin_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
) -> Result<(String, bool), CommandError> {
    todo!()
}

/// Devuelve una tupla de dos HashMaps. El primero contiene las líneas que no cambiaron en "otro"
/// y el segundo contiene las diferencias entre "otro" y "común".\
/// Las diferencias están representadas por una tupla de dos vectores de strings. El primer vector
/// contiene las líneas nuevas lineas de "otro" y el segundo vector contiene las líneas de "común"
/// que cambiaron en "otro".
fn get_diffs(
    common_content: &String,
    other_content: &String,
) -> Result<
    (
        HashMap<usize, String>,
        HashMap<usize, (Vec<String>, Vec<String>)>,
    ),
    CommandError,
> {
    let mut common_not_changed_in_other = HashMap::<usize, String>::new();
    let mut other_diffs = HashMap::<usize, (Vec<String>, Vec<String>)>::new(); // index, (new_lines, discarted_lines)
    let common_lines: Vec<String> = common_content.lines().collect::<Vec<&str>>();
    let other_lines = other_content.lines().collect::<Vec<&str>>();
    let mut common_index = 0;
    let mut other_index = 0;
    let mut common_buf = Vec::<String>::new();
    let mut other_buf = Vec::<String>::new();

    loop {
        let mut common_line_op = get_element(&common_lines, common_index);
        let mut other_line_op = get_element(&other_lines, other_index);
        if common_line_op.is_none() && other_line_op.is_none() {
            break;
        }
        if common_line_op == other_line_op {
            common_not_changed_in_other.insert(
                common_index,
                common_line_op
                    .ok_or(CommandError::MergeConflict("Error imposible".to_string()))?
                    .to_string(),
            );
            common_index += 1;
            other_index += 1;
        } else {
            let first_diff_other_index = other_index;
            let first_diff_common_index = common_index;
            loop {
                if let Some(common_line) = &common_line_op {
                    if let Some(other_line_index) = other_buf
                        .iter()
                        .position(|other_line| other_line == common_line)
                    {
                        let new_lines = other_buf[..other_line_index].to_vec();
                        let discarted_lines = common_buf.clone();
                        other_index = first_diff_other_index + new_lines.len();
                        other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                        break;
                    }
                    common_buf.push(common_line.to_string());
                }
                if let Some(other_line) = &other_line_op {
                    if let Some(common_line_index) = common_buf
                        .iter()
                        .position(|common_line| common_line == other_line)
                    {
                        let new_lines = other_buf.clone();
                        let discarted_lines = common_buf[..common_line_index].to_vec();
                        common_index = first_diff_common_index + discarted_lines.len();
                        other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                        break;
                    }
                    other_buf.push(other_line.to_string());
                }
                if common_line_op.is_none() && other_line_op.is_none() {
                    let new_lines = other_buf.clone();
                    let discarted_lines = common_buf.clone();
                    common_index = first_diff_common_index + discarted_lines.len();
                    other_index = first_diff_other_index + new_lines.len();
                    other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                    break;
                }

                if common_index < common_lines.len() {
                    common_index += 1;
                }
                if other_index < other_lines.len() {
                    other_index += 1;
                }
                common_line_op = get_element(&common_lines, common_index);
                other_line_op = get_element(&other_lines, other_index);
            }
            common_buf.clear();
            other_buf.clear();
        }
    }

    Ok((common_not_changed_in_other, other_diffs))
}

fn get_element(vector: &Vec<&str>, index: usize) -> Option<String> {
    if index >= vector.len() {
        None
    } else {
        Some(vector[index].to_string())
    }
}
*/
