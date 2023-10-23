use std::{collections::HashMap, fmt::Write};

use crate::{commands::command_errors::CommandError, logger::Logger};

use super::{changes_controller::ChangesController, changes_types::ChangeType, format::Format};

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
    ) {
        //let output_message = format!("On branch {}\nChanges to be committed:\n  (use "git {} to unstage)", branch, "git restore --staged <file>...");
    }
}
