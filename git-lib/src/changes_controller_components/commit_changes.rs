use std::{collections::HashMap, fs};

use crate::objects::git_object::GitObjectTrait;
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
    ) -> Result<(usize, usize, usize), CommandError> {
        let (files_changed, insertions, deletions) = Self::get_changes_insertions_deletions(
            staging_area_changes,
            db,
            logger,
            commit_tree,
            changes_to_be_commited,
        )?;
        Ok((files_changed, insertions, deletions))
    }

    fn get_changes_insertions_deletions(
        staging_area_changes: HashMap<String, String>,
        db: &ObjectsDatabase,
        logger: &mut Logger,
        commit_tree: Option<Tree>,
        changes_to_be_commited: &HashMap<String, ChangeType>,
    ) -> Result<(usize, usize, usize), CommandError> {
        let mut insertions = 0;
        let mut deletions = 0;
        let mut files_changed = 0;
        for (path, change_type) in changes_to_be_commited.iter() {
            logger.log(&format!("Checking for insertions/deletions: {}", path));
            match change_type {
                ChangeType::Added | ChangeType::Renamed => {
                    insertions += count_insertions(path)?;
                    files_changed += 1;
                }
                ChangeType::Deleted => {
                    if let Some(tree) = &commit_tree {
                        deletions += count_deletions(path, logger, tree)?;
                        files_changed += 1;
                    }
                }
                _ => {}
            }
        }

        Ok((files_changed, insertions, deletions))
    }
}

fn count_insertions(path: &str) -> Result<usize, CommandError> {
    let content =
        fs::read_to_string(path).map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let lines: Vec<&str> = content.lines().collect();
    Ok(lines.len())
}

fn count_deletions(
    path: &str,
    logger: &mut Logger,
    commit_tree: &Tree,
) -> Result<usize, CommandError> {
    let Some(mut object) = commit_tree.get_blob(path, logger) else {
        return Err(CommandError::FileNameError);
    };
    let Some(blob) = object.as_mut_blob() else {
        return Err(CommandError::UnknownObjectType);
    };
    let content = blob.content()?;
    let content_str: String = format!("{}", String::from_utf8_lossy(&content));
    let lines: Vec<&str> = content_str.lines().collect();
    Ok(lines.len())
}

fn read_contents(db: &ObjectsDatabase, commit_tree: Option<Tree>) {}

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
    let common_lines: Vec<&str> = common_content.lines().collect::<Vec<&str>>();
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
