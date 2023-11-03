// use crate::command_errors::CommandError;
// use crate::logger::Logger;
// use crate::objects_database::ObjectsDatabase;

// use super::tree::Tree;

// pub fn is_in_last_commit(
//     db: &ObjectsDatabase,
//     blob_hash: String,
//     logger: &mut Logger,
// ) -> Result<(bool, String), CommandError> {
//     if let Some(mut tree) = build_last_commit_tree(db, logger)? {
//         return Ok(tree.has_blob_from_hash(&blob_hash, logger)?);
//     }
//     Ok((false, "".to_string()))
// }

// pub fn build_last_commit_tree(
//     db: &ObjectsDatabase,
//     logger: &mut Logger,
// ) -> Result<Option<Tree>, CommandError> {
//     if let Some(tree) = get_commit_tree(db, logger)? {
//         return Ok(Some(tree));
//     }
//     Ok(None)
// }
