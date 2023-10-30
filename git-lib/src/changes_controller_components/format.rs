use crate::logger::Logger;
use crate::objects_database::ObjectsDatabase;
use crate::{command_errors::CommandError, objects::last_commit::build_last_commit_tree};
use std::{collections::HashMap, io::Write};

use super::{changes_controller::ChangesController, changes_types::ChangeType};
pub trait Format {
    fn show(
        &self,
        db: &ObjectsDatabase,
        logger: &mut Logger,
        output: &mut dyn Write,
        branch: &str,
        commit_output: bool,
    ) -> Result<(), CommandError> {
        let commit_tree = build_last_commit_tree(db, logger)?;
        let initial_commit = {
            if commit_tree.is_none() {
                true
            } else {
                false
            }
        };
        let changes_controller = ChangesController::new(db, logger, commit_tree)?;
        let changes_to_be_commited = changes_controller.get_changes_to_be_commited();
        let changes_not_staged = changes_controller.get_changes_not_staged();
        let untracked_files = changes_controller.get_untracked_files();
        self.get_status(
            logger,
            output,
            changes_to_be_commited,
            changes_not_staged,
            untracked_files,
            (branch, commit_output, initial_commit),
        )?;
        Ok(())
    }

    fn get_status(
        &self,
        logger: &mut Logger,
        output: &mut dyn Write,
        changes_to_be_commited: &HashMap<String, ChangeType>,
        changes_not_staged: &HashMap<String, ChangeType>,
        untracked_files: &Vec<String>,
        long_info: (&str, bool, bool),
    ) -> Result<(), CommandError>;
}

/*
-Short o Lonf
*/

/*

Short:

status of the index and Y shows the status of the working tree.

X          Y     Meaning
-------------------------------------------------
            [AMD]   not updated
M        [ MTD]  updated in index
T        [ MTD]  type changed in index
A        [ MTD]  added to index
D                deleted from index
R        [ MTD]  renamed in index
C        [ MTD]  copied in index
[MTARC]          index and work tree matches
[ MTARC]    M    work tree changed since index
[ MTARC]    T    type changed in work tree since index
[ MTARC]    D    deleted in work tree
            R    renamed in work tree
            C    copied in work tree
-------------------------------------------------
D           D    unmerged, both deleted
A           U    unmerged, added by us
U           D    unmerged, deleted by them
U           A    unmerged, added by them
D           U    unmerged, deleted by us
A           A    unmerged, both added
U           U    unmerged, both modified
-------------------------------------------------
?           ?    untracked
!           !    ignored
*/

/*

Long:

On branch status
Your branch is up to date with 'origin/status'.

Changes to be committed:
  (use "git restore --staged <file>..." to unstage)
    modified:   src/commands/status_components/status.rs

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
    modified:   src/commands/status_components/status.rs

----ej2---


On branch status
Your branch is up to date with 'origin/status'.

Changes to be committed:
  (use "git restore --staged <file>..." to unstage)
    modified:   src/main.rs

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
    modified:   src/commands/status_components/format.rs
    modified:   src/commands/status_components/mod.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
    src/commands/status_components/changes_types.rs
    src/commands/status_components/long_format.rs
    src/commands/status_components/merge_conflicts.rs
    src/commands/status_components/short_code.rs
    src/commands/status_components/short_format.rs
    src/commands/status_components/staging_area.rs
    src/commands/status_components/working_tree.rs

*/
