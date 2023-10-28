use std::{collections::HashMap, io::Write};

use crate::command_errors::CommandError;
use crate::logger::Logger;

use super::{changes_types::ChangeType, format::Format};

pub struct LongFormat;

impl Format for LongFormat {
    fn get_status(
        &self,
        logger: &mut Logger,
        output: &mut dyn Write,
        changes_to_be_commited: &HashMap<String, ChangeType>,
        changes_not_staged: &HashMap<String, ChangeType>,
        untracked_files: &Vec<String>,
        branch: &str,
    ) -> Result<(), CommandError> {
        let mut output_message = format!("On branch {}", branch);
        let changes_to_be_commited = sort_hashmap(changes_to_be_commited);
        if !changes_to_be_commited.is_empty() {
            output_message = format!(
                "{}\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n", output_message
            );
        }
        for (path, change_type) in changes_to_be_commited.iter() {
            let change = change_type.get_long_type();
            logger.log(&format!("Change to be commited: {}", path));
            output_message = format!("{}	{}:   {}\n", output_message, change, path);
        }
        let changes_not_staged = sort_hashmap(changes_not_staged);
        if !changes_not_staged.is_empty() {
            output_message = format!("{}\nChanges not staged for commit:\n  (use \"git add/rm <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n", output_message);
        }
        for (path, change_type) in changes_not_staged.iter() {
            let change = change_type.get_long_type();

            output_message = format!("{}\t{}:   {}\n", output_message, change, path);
        }

        let untracked_files = sort_vector(untracked_files);
        if !untracked_files.is_empty() {
            output_message = format!("{}\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n", output_message);
        }

        for path in untracked_files.iter() {
            output_message = format!("{}	{}\n", output_message, path);
        }

        if changes_to_be_commited.is_empty()
            && changes_not_staged.is_empty()
            && untracked_files.is_empty()
        {
            output_message = format!("{}\nnothing to commit, working tree clean", output_message);
        } else if changes_to_be_commited.is_empty() {
            output_message = format!(
                "{}\nno changes added to commit (use \"git add\" and/or \"git commit -a\"",
                output_message
            );
        }
        writeln!(output, "{}", output_message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

fn sort_hashmap(files: &HashMap<String, ChangeType>) -> Vec<(String, ChangeType)> {
    let mut keys: Vec<&String> = files.keys().collect();
    keys.sort();

    let mut sorted_files: Vec<(String, ChangeType)> = Vec::new();
    for key in keys {
        if let Some(value) = files.get(key) {
            if !matches!(value, ChangeType::Unmodified) {
                sorted_files.push((key.clone(), value.clone()));
            }
        }
    }
    sorted_files
}

fn sort_vector(files: &Vec<String>) -> Vec<String> {
    let mut files = files.to_owned();
    files.sort();
    files
}
