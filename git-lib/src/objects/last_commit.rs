use crate::command_errors::CommandError;
use crate::{
    logger::Logger,
    {branch_manager::get_last_commit, objects_database},
};

use super::{git_object::GitObjectTrait, tree::Tree};

pub fn is_in_last_commit(
    blob_hash: String,
    logger: &mut Logger,
) -> Result<(bool, String), CommandError> {
    if let Some(mut tree) = build_last_commit_tree(logger)? {
        return Ok(tree.has_blob_from_hash(&blob_hash, logger)?);
    }
    Ok((false, "".to_string()))
}

pub fn build_last_commit_tree(logger: &mut Logger) -> Result<Option<Tree>, CommandError> {
    if let Some(tree) = get_commit_tree(logger)? {
        return Ok(Some(tree));
    }
    Ok(None)
}

pub fn get_commit_tree(logger: &mut Logger) -> Result<Option<Tree>, CommandError> {
    let Some(last_commit) = get_last_commit()? else {
        return Ok(None);
    };
    logger.log(&format!("Last commit : {}", last_commit));

    let mut commit_box = objects_database::read_object(&last_commit, logger)?;
    if let Some(commit) = commit_box.as_mut_commit() {
        logger.log(&format!(
            "Last commit content : {}",
            String::from_utf8_lossy(&commit.content()?)
        ));
        let tree = commit.get_tree();

        logger.log(&format!(
            "tree content : {}",
            String::from_utf8_lossy(&(tree.to_owned().content()?))
        ));
        return Ok(Some(tree.to_owned()));
    }
    Ok(None)
}
