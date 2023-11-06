use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    io::Cursor,
    option,
};

use crate::{
    command_errors::CommandError,
    logger::Logger,
    objects::{commit_object::CommitObject, git_object::get_type_and_len},
    objects_database::ObjectsDatabase,
};

pub fn get_analysis(
    local_branches: Vec<(String, String)>,
    db: ObjectsDatabase,
    refs_hash: HashMap<String, String>,
    logger: &mut Logger,
) -> Result<
    (
        HashMap<String, (String, String)>,
        HashMap<String, (CommitObject, Option<String>)>,
    ),
    CommandError,
> {
    let mut hash_branch_status = HashMap::<String, (String, String)>::new(); // HashMap<branch, (old_hash, new_hash)>
    let mut commits_map = HashMap::<String, (CommitObject, Option<String>)>::new(); // HashMap<hash, (CommitObject, Option<branch>)>

    for (local_branch, local_hash) in local_branches {
        logger.log("Looping");
        logger.log(&format!(
            "local_branch: {}, local_hash: {}\n",
            &local_branch, &local_hash
        ));
        let remote_hash = match refs_hash.get(&local_branch) {
            Some(remote_hash) => remote_hash.clone(),
            None => "0000000000000000000000000000000000000000".to_string(),
        };

        if local_hash == *remote_hash {
            logger.log("Local branch is up-to-date");
            continue;
        }
        let hash_to_look_for = HashSet::<String>::from_iter(vec![remote_hash.clone()]);
        rebuild_commits_tree(
            &db,
            &local_hash,
            &mut commits_map,
            Some(local_branch.to_string()),
            false,
            &hash_to_look_for,
            true,
            logger,
        )?;

        logger.log(&format!(
            "local_branch: {}, local_hash: {}\n",
            &local_branch, &local_hash
        ));

        if let Some((_, Some(remote_branch))) = commits_map.get(&remote_hash) {
            if remote_branch == &local_branch {
                hash_branch_status
                    .insert(local_branch.to_string(), (remote_hash.clone(), local_hash));
            } else {
                return Err(CommandError::PushBranchBehind(local_branch.to_owned()));
            }
        } else {
            return Err(CommandError::PushBranchBehind("".to_string()));
        }
        commits_map.remove(&remote_hash);
    }

    Ok((hash_branch_status, commits_map))
}

/// Reconstruye el arbol de commits que le preceden a partir de un commit
pub fn rebuild_commits_tree(
    db: &ObjectsDatabase,
    hash_commit: &String,
    commits_map: &mut HashMap<String, (CommitObject, Option<String>)>, // HashMap<hash, (commit, branch)>
    branch: Option<String>,
    log_all: bool,
    hash_to_look_for: &HashSet<String>,
    build_tree: bool,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    logger.log("rebuild_commits_tree");
    if commits_map.contains_key(&hash_commit.to_string()) {
        return Ok(());
    }

    logger.log(&format!("Reading file : {}", hash_commit));
    let (_, decompressed_data) = db.read_file(hash_commit, logger)?;
    logger.log(&format!(
        "decompressed_data: {}",
        String::from_utf8_lossy(&decompressed_data)
    ));

    let mut stream = Cursor::new(decompressed_data);

    let (string, len) = get_type_and_len(&mut stream)?;

    logger.log(&format!("string: {}, len: {}", string, len));

    let option_db = if build_tree { Some(db) } else { None };
    let mut commit_object_box =
        CommitObject::read_from(option_db, &mut stream, logger, Some(hash_commit.clone()))?;

    logger.log(&format!(
        "commit_object_box: {:?}",
        commit_object_box.content(None),
    ));

    // println!("commit_object_box: {:?}", commit_object_box.content());
    //get_type_and_len(&mut stream)?;

    // let mut commit_object = read_from_for_log(&db, &mut stream, &mut logger.logger, hash_commit)?;

    // println!("commit_object: {:?}", commit_object.content());

    let Some(commit_object) = commit_object_box.as_mut_commit() else {
        return Err(CommandError::InvalidCommit);
    };

    if hash_to_look_for.contains(hash_commit) {
        let commit_with_branch = (commit_object.to_owned(), branch);
        commits_map.insert(hash_commit.to_string(), commit_with_branch);
        return Ok(());
    }

    let parents_hash = commit_object.get_parents();

    if parents_hash.len() > 0 {
        let principal_parent = &parents_hash[0];
        rebuild_commits_tree(
            db,
            &principal_parent,
            commits_map,
            branch.clone(),
            log_all,
            hash_to_look_for,
            build_tree,
            logger,
        )?;

        if !log_all {
            for parent_hash in parents_hash.iter().skip(1) {
                for hash_to_look_for_one in hash_to_look_for.iter() {
                    if commits_map.contains_key(&hash_to_look_for_one.to_string()) {
                        return Ok(());
                    }
                }
                rebuild_commits_tree(
                    db,
                    &parent_hash,
                    commits_map,
                    None,
                    log_all,
                    hash_to_look_for,
                    build_tree,
                    logger,
                )?;
            }
        }
    }

    if commits_map.contains_key(&hash_commit.to_string()) {
        return Ok(());
    }

    let commit_with_branch = (commit_object.to_owned(), branch);
    commits_map.insert(hash_commit.to_string(), commit_with_branch);
    Ok(())
}
