use crate::command_errors::CommandError;
use crate::logger::Logger;
use std::{collections::HashMap, io::Write};

use super::{change_object::ChangeObject, changes_types::ChangeType, format::Format};

pub struct ShortFormat;

impl Format for ShortFormat {
    fn get_status(
        &self,
        logger: &mut Logger,
        output: &mut dyn Write,
        changes_to_be_commited: &HashMap<String, ChangeType>,
        changes_not_staged: &HashMap<String, ChangeType>,
        untracked_files: &Vec<String>,
        _: &str,
    ) -> Result<(), CommandError> {
        let mut changes: HashMap<String, ChangeObject> = HashMap::new();
        logger.log(&format!(
            "Len changes: {}, {}, {}",
            changes_to_be_commited.len(),
            changes_not_staged.len(),
            untracked_files.len()
        ));
        for (path, index_status) in changes_to_be_commited.iter() {
            let change: ChangeObject;
            if let Some(working_tree_status) = changes_not_staged.get(path) {
                if matches!(index_status, ChangeType::Unmodified)
                    && matches!(working_tree_status, ChangeType::Unmodified)
                {
                    continue;
                }
                change = ChangeObject::new(
                    path.to_string(),
                    working_tree_status.to_owned(),
                    index_status.to_owned(),
                );
            } else {
                change = ChangeObject::new(
                    path.to_string(),
                    ChangeType::Unmodified,
                    index_status.to_owned(),
                );
            }
            _ = changes.insert(path.to_string(), change);
        }

        for (path, working_tree_status) in changes_not_staged.iter() {
            if matches!(working_tree_status, ChangeType::Unmodified) {
                continue;
            }
            if changes_to_be_commited.get(path).is_none() {
                let change: ChangeObject = ChangeObject::new(
                    path.to_string(),
                    working_tree_status.to_owned(),
                    ChangeType::Unmodified,
                );
                _ = changes.insert(path.to_string(), change);
            }
        }
        for path in untracked_files.iter() {
            let change = ChangeObject::new(
                path.to_string(),
                ChangeType::Untracked,
                ChangeType::Untracked,
            );
            _ = changes.insert(path.to_string(), change);
        }
        let mut output_message = "".to_string();
        let changes = sort_changes(changes);
        for change in changes.iter() {
            output_message = format!("{}{}\n", output_message, change.to_string_change());
        }
        logger.log(&format!("Output message: {}", output_message));
        if !output_message.is_empty() {
            write!(output, "{}", output_message)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }
}

fn sort_changes(changes: HashMap<String, ChangeObject>) -> Vec<ChangeObject> {
    let mut keys: Vec<&String> = changes.keys().collect();
    keys.sort();

    let mut sorted_changes: Vec<ChangeObject> = Vec::new();
    for key in keys {
        if let Some(value) = changes.get(key) {
            sorted_changes.push(value.clone());
        }
    }
    sorted_changes
}
