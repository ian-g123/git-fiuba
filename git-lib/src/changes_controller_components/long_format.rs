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
        (branch, commit_output, initial_commit): (&str, bool, bool),
        unmerged_paths: &HashMap<String, ChangeType>,
        merge: bool,
        branches_diverge_info: (bool, usize, usize),
    ) -> Result<(), CommandError> {
        let mut output_message = format!("On branch {}", branch);
        if initial_commit {
            if commit_output {
                output_message = format!("{}\n\nInitial commit\n", output_message)
            } else {
                output_message = format!("{}\n\nNo commits yet\n", output_message)
            }
        }
        let (remote_exists, ahead, behind) = branches_diverge_info;
        if remote_exists {
            output_message += &format!("{}", set_diverge_message(ahead, behind, &branch));
        }
        if merge {
            output_message += &format!("{}", set_unmerged_message(unmerged_paths));
        }

        let changes_to_be_commited = sort_hashmap_and_filter_unmodified(changes_to_be_commited);
        if !changes_to_be_commited.is_empty() {
            output_message = format!(
                "{}\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n", output_message
            );
        }
        for (path, change_type) in changes_to_be_commited.iter() {
            let change = change_type.get_long_type();
            output_message = format!("{}	{}:   {}\n", output_message, change, path);
        }
        let changes_not_staged = sort_hashmap_and_filter_unmodified(changes_not_staged);
        if !changes_not_staged.is_empty() {
            output_message = format!("{}\nChanges not staged for commit:\n  (use \"git add/rm <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n", output_message);
        }
        for (path, change_type) in changes_not_staged.iter() {
            let change = change_type.get_long_type();

            output_message = format!("{}\t{}:   {}\n", output_message, change, path);
        }

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
            if initial_commit {
                output_message = format!(
                    "{}\nnothing to commit (create/copy files and use \"git add\" to track)",
                    output_message
                );
            } else {
                output_message =
                    format!("{}\nnothing to commit, working tree clean", output_message);
            }
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

pub fn sort_hashmap_and_filter_unmodified(
    files: &HashMap<String, ChangeType>,
) -> Vec<(String, ChangeType)> {
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

fn set_unmerged_message(unmerged_paths: &HashMap<String, ChangeType>) -> String {
    let mut message = String::new();
    if unmerged_paths.is_empty() {
        message += &format!("\nAll conflicts fixed but you are still merging.\n  (use \"git commit\" to conclude merge)\n");
        return message;
    }
    let unmerged_paths = sort_hashmap_and_filter_unmodified(unmerged_paths);
    message += &format!("\nYou have unmerged paths.\n  (fix conflicts and run \"git commit\")\n  (use \"git merge --abort\" to abort the merge)\n\n");
    message += &format!(
        "Unmerged paths:\n  (use \"git add/rm <file>...\" as appropriate to mark resolution)\n"
    );

    for (path, change_type) in unmerged_paths.iter() {
        let change = change_type.get_long_type();
        message += &format!("\t{}:   {}\n", change, path);
    }
    message
}

pub fn set_diverge_message(ahead: usize, behind: usize, branch: &str) -> String {
    let mut message = String::new();
    if ahead == 0 && behind == 0 {
        message += &format!("\nYour branch is up to date with 'origin/{}'.\n", branch);
    } else if ahead == 0 {
        let plural = if behind == 1 { "" } else { "s" };
        message += &format!("\nYour branch is behind 'origin/{}' by {} commit{}, and can be fast-forwarded.\n  (use \"git pull\" to update your local branch)", branch, behind, plural);
    } else if behind == 0 {
        let plural = if ahead == 1 { "" } else { "s" };
        message += &format!("\nYour branch is ahead 'origin/{}' by {} commit{}.\n  (use \"git push\" to publish tour local commits)\n", branch, ahead, plural);
    } else {
        message += &format!("\nYour branch and 'origin/{}' have diverged, and have {} and {} different commits each, respectively.\n  (use \"git pull\" to merge the remote branch into yours)\n", branch, ahead, behind);
    }
    message
}
