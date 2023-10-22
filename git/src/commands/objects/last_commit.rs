use std::io::Cursor;

use crate::{
    commands::{branch_manager::get_last_commit, command_errors::CommandError, objects_database},
    logger::Logger,
};

use super::{blob::Blob, git_object, mode::Mode, tree::Tree};

pub fn is_in_last_commit(
    blob_hash: String,
    logger: &mut Logger,
) -> Result<(bool, String), CommandError> {
    if let Some(tree) = build_last_commit_tree(logger)? {
        return Ok(tree.has_blob_from_hash(&blob_hash)?);
    }
    Ok((false, "".to_string()))
}

pub fn build_last_commit_tree(logger: &mut Logger) -> Result<Option<Tree>, CommandError> {
    if let Some(hash) = get_commit_tree_hash(logger)? {
        let mut tree = Tree::new("".to_string());
        build_tree(&mut tree, &hash, logger)?;
        return Ok(Some(tree));
    }
    Ok(None)
}

fn build_tree(tree: &mut Tree, hash: &str, logger: &mut Logger) -> Result<(), CommandError> {
    let mut data: Vec<u8> = Vec::new();
    let mut stream: Cursor<&mut Vec<u8>> = Cursor::new(&mut data);
    git_object::display_from_hash(&mut data, &hash, logger)?;
    let buf = String::from_utf8_lossy(&data).to_string();
    let lines: Vec<&str> = buf.split_terminator("\n").collect();
    for line in lines {
        let info: Vec<&str> = line.split_terminator(" ").collect();

        let (mode, obj_type, this_hash, name) = (
            info[0],
            info[1].to_string(),
            info[2].to_string(),
            info[info.len() - 1].to_string(),
        );

        if obj_type == "blob" {
            let mode = Mode::read_from_string(mode)?;
            let blob = Blob::new_from_hash_and_name(this_hash.to_string(), name.clone(), mode)?;
            tree.add_object(name, Box::new(blob));
        } else {
            let mut new_tree = Tree::new(name.clone());
            build_tree(&mut new_tree, &this_hash, logger)?;
            tree.add_object(name, Box::new(new_tree));
        }
    }
    Ok(())
}

pub fn get_commit_tree_hash(logger: &mut Logger) -> Result<Option<String>, CommandError> {
    let Some(last_commit) = get_last_commit()? else {
        return Ok(None);
    };
    let mut commit_box = objects_database::read_object(&last_commit, logger)?;
    if let Some(mut commit) = commit_box.as_commit_mut() {
        let tree_hash = commit.to_owned().get_tree_hash()?;
        return Ok(Some(tree_hash));
    }
    Ok(None)
}
