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
        HashMap<String, (CommitObject, usize, usize)>,
    ),
    CommandError,
> {
    let mut hash_branch_status = HashMap::<String, (String, String)>::new(); // HashMap<branch, (old_hash, new_hash)>
    let mut commits_map = HashMap::<String, (CommitObject, usize, usize)>::new(); // HashMap<hash, (CommitObject, Option<branch>)>

    for (i, (local_branch, local_hash)) in local_branches.into_iter().enumerate() {
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
            false,
            &hash_to_look_for,
            true,
            logger,
            i,
        )?;

        if commits_map.contains_key(&remote_hash) {
            hash_branch_status.insert(local_branch.to_string(), (remote_hash.clone(), local_hash));
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
    commits_map: &mut HashMap<String, (CommitObject, usize, usize)>, // HashMap<hash, (commit, branch)>
    log_all: bool,
    hash_to_look_for: &HashSet<String>,
    build_tree: bool,
    logger: &mut Logger,
    color_idx: usize,
) -> Result<usize, CommandError> {
    if let Some((_, _, depth)) = commits_map.get(&hash_commit.to_string()) {
        return Ok(depth.clone());
    }

    logger.log(&format!(
        "BUILD COMMIT TREE Reading file db.read_file: {}",
        hash_commit
    ));
    let (type_str, len, content) = db.read_object_data(hash_commit, logger)?;

    logger.log(&format!("type_str: {}, len: {}", type_str, len));

    let option_db = if build_tree { Some(db) } else { None };
    let mut commit_object_box = CommitObject::read_from(
        option_db,
        &mut content.as_slice(),
        logger,
        Some(hash_commit.clone()),
    )?;

    logger.log(&format!(
        "commit_object_box: {:?}",
        commit_object_box.content(None),
    ));

    let Some(commit_object) = commit_object_box.as_mut_commit() else {
        return Err(CommandError::InvalidCommit);
    };

    if hash_to_look_for.contains(hash_commit) {
        let commit_with_branch = (commit_object.to_owned(), color_idx, 0);
        commits_map.insert(hash_commit.to_string(), commit_with_branch);
        return Ok(0);
    }

    let parents_hash = commit_object.get_parents();
    logger.log(&format!(
        "parents_hash: {:?} of the commit: {}",
        parents_hash, hash_commit
    ));
    // if parents_hash.len() > 0 {
    //     let principal_parent = &parents_hash[0];
    //     logger.log(&format!(
    //         "principal parent hash : {} of the commit: {}",
    //         principal_parent, hash_commit
    //     ));
    //     let mut depth = rebuild_commits_tree(
    //         db,
    //         &principal_parent,
    //         commits_map,
    //         branch.clone(),
    //         log_all,
    //         hash_to_look_for,
    //         build_tree,
    //         logger,
    //     )?;
    let mut max_depth = 0;
    for (i_color, parent_hash) in parents_hash.iter().enumerate() {
        for hash_to_look_for_one in hash_to_look_for.iter() {
            if let Some((_, _, depth)) = commits_map.get(&hash_to_look_for_one.to_string()) {
                return Ok(depth.clone());
            }
        }
        let depth = rebuild_commits_tree(
            db,
            &parent_hash,
            commits_map,
            log_all,
            hash_to_look_for,
            build_tree,
            logger,
            color_idx + i_color,
        )?;
        if depth > max_depth {
            max_depth = depth;
        }
    }

    if let Some((_, _, depth)) = commits_map.get(&hash_commit.to_string()) {
        return Ok(depth.clone());
    }

    let commit_with_branch = (commit_object.to_owned(), color_idx, max_depth + 1);
    commits_map.insert(hash_commit.to_string(), commit_with_branch);
    Ok(max_depth + 1)
}

/// Reconstruye el arbol de commits que le preceden a partir de un commit
pub fn get_parents_hash_map(
    hash_commit: &String,
    commits_map: &mut HashMap<String, (CommitObject, Option<String>)>, // HashMap<hash, (commit, branch)>
    parents_hash: &mut HashMap<String, HashSet<String>>,
    sons_hash: &mut HashMap<String, HashSet<String>>,
    //logger: &mut Logger,
) -> Result<(), CommandError> {
    println!("MMM{}", hash_commit);
    // if parents_hash.contains_key(&hash_commit.to_string()) {
    //     return Ok(());
    // }

    let commit_object = match commits_map.get_mut(hash_commit) {
        Some(commit_object_box_aux) => commit_object_box_aux.0.to_owned(),
        None => return Ok(()),
    };

    let parents_vec: Vec<String> = commit_object.get_parents();
    println!("padres {:?}", parents_vec);

    for parent_hash in parents_vec.iter() {
        let hash_set_p = parents_hash
            .entry(hash_commit.to_string())
            .or_insert(HashSet::new());
        hash_set_p.insert(parent_hash.to_string());

        let hash_set_s = sons_hash
            .entry(parent_hash.to_string())
            .or_insert(HashSet::new());
        hash_set_s.insert(hash_commit.to_string());

        let hash_set_s_s = sons_hash
            .entry(hash_commit.to_string())
            .or_insert(HashSet::new());

        for childs in hash_set_s_s.iter() {
            let hash_set_p_s = parents_hash
                .entry(childs.to_string())
                .or_insert(HashSet::new());
            hash_set_p_s.insert(parent_hash.to_string());
            //parents_hash.insert(childs.to_string(), hash_set_p_s.to_owned());
        }
        get_parents_hash_map(parent_hash, commits_map, parents_hash, sons_hash)?;
    }

    Ok(())
}
