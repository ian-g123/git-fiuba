use std::{collections::HashMap, io::Write};

use crate::{
    command_errors::CommandError,
    logger::Logger,
    objects::{mode::Mode, tree::Tree},
    objects_database::ObjectsDatabase,
    staging_area::StagingArea,
};

use super::{
    changes_controller::ChangesController, changes_types::ChangeType,
    commit_changes::CommitChanges, long_format::sort_hashmap_and_filter_unmodified,
};

pub struct CommitFormat;

impl CommitFormat {
    pub fn show(
        staging_area: &StagingArea,
        db: &ObjectsDatabase,
        logger: &mut Logger,
        commit_tree: Option<Tree>,
        hash: &str,
        branch_name: &str,
        message: &str,
        output: &mut dyn Write,
        git_path: &str,
        working_dir: &str,
    ) -> Result<(), CommandError> {
        let changes_controller = ChangesController::new(
            db,
            git_path,
            working_dir,
            logger,
            commit_tree.clone(),
            staging_area,
        )?;
        let changes_to_be_commited = changes_controller.get_changes_to_be_commited();
        let (files_changed, insertions, deletions) = CommitChanges::new(
            staging_area.get_files(),
            db,
            logger,
            commit_tree.clone(),
            changes_to_be_commited,
        )?;
        let is_root = if commit_tree.is_none() { true } else { false };
        let changes_to_be_commited = sort_hashmap_and_filter_unmodified(changes_to_be_commited);
        let output_message = get_commit_sucess_message(
            &changes_to_be_commited,
            files_changed,
            insertions,
            deletions,
            hash,
            branch_name,
            message,
            is_root,
        )?;
        logger.log("before output commit");

        write!(output, "{}", output_message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        logger.log("after output commit");
        Ok(())
    }
}

fn get_commit_sucess_message(
    changes_to_be_commited: &Vec<(String, ChangeType)>,
    files_changed: usize,
    insertions: usize,
    deletions: usize,
    hash: &str,
    branch_name: &str,
    message: &str,
    is_root: bool,
) -> Result<String, CommandError> {
    let mut output_message = format!("[{} ", branch_name);
    if is_root {
        output_message += &format!("(root-commit) ");
    }
    let message_vec: Vec<&str> = message.lines().collect();
    let message = message_vec.join(" ");
    output_message += &format!("{}] {}\n", hash[..7].to_string(), message);
    output_message += &format!(" {} file", files_changed);
    if files_changed > 1 {
        output_message += "s";
    }
    output_message += &format!(" changed");
    if insertions > 0 {
        if insertions == 1 {
            output_message += &format!(", {} insertion(+)", insertions);
        } else {
            output_message += &format!(", {} insertions(+)", insertions);
        }
    }
    if deletions > 0 {
        if deletions == 1 {
            output_message += &format!(", {} deletion(-)", deletions);
        } else {
            output_message += &format!(", {} deletions(-)", deletions);
        }
    }
    if insertions == 0 && deletions == 0 {
        output_message += ", 0 insertions(+), 0 deletions(-)";
    }
    output_message += "\n";
    let mode = Mode::RegularFile;
    let mode_str = mode.to_string();
    for (path, type_change) in changes_to_be_commited.iter() {
        match type_change {
            ChangeType::Added | ChangeType::Renamed => {
                output_message += &format!(" create mode {} {}\n", mode_str, path)
            }
            ChangeType::Deleted => {
                output_message += &format!(" delete mode {} {}\n", mode_str, path)
            }
            _ => {}
        }
    }
    Ok(output_message)
}
