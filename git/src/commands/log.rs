use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write, Cursor},
};

use crate::commands::command::Command;
use git_lib::{
    command_errors::CommandError,
    git_repository::{get_head_ref, local_branches},
    logger::Logger,
    objects::commit_object::CommitObject,
    objects_database::read_file,
};

use super::command::ConfigAdderFunction;

/// Commando log
pub struct Log {
    all: bool,
}

impl Command for Log {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "log" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        let mut commits = instance.run()?;
        // print_for_log(output, &mut commits)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![Self::add_all_config]
    }
}

impl Log {
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut log = Self::new_default();
        log.config(args)?;
        Ok(log)
    }

    fn new_default() -> Self {
        Self { all: false }
    }

    fn add_all_config(log: &mut Log, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--all" {
            return Err(CommandError::WrongFlag);
        }
        log.all = true;
        Ok(i + 1)
    }

    fn run(&self) -> Result<Vec<(CommitObject, Option<String>)>, CommandError> {
        let mut branches_with_their_last_hash: Vec<(String, String)> = Vec::new(); // branch, hashes
        let mut commits_map: HashMap<String, (CommitObject, Option<String>)> = HashMap::new(); //hash, (commit, branch)

        if self.all {
            push_branch_hashes(&mut branches_with_their_last_hash)?;
        } else {
            let current_branch = get_head_ref()?;
            let hash_commit = get_commit_hash(&current_branch)?;
            let branch_and_hash = (current_branch, hash_commit);
            branches_with_their_last_hash.push(branch_and_hash);
        }

        for branch_with_commit in branches_with_their_last_hash {
            self.rebuild_commits_tree(
                &branch_with_commit.1,
                &mut commits_map,
                Some(branch_with_commit.0),
            )?;
        }
        let mut commits: Vec<_> = commits_map.drain().map(|(_, v)| v).collect();

        // sort_commits_descending_date(&mut commits);

        Ok(commits)
    }

    /// Reconstruye el arbol de commits que le preceden a partir de un commit
    fn rebuild_commits_tree(
        &self,
        hash_commit: &String,
        commits_map: &mut HashMap<String, (CommitObject, Option<String>)>,
        branch: Option<String>,
    ) -> Result<(), CommandError> {
        if commits_map.contains_key(hash_commit) {
            return Ok(());
        }

        let logger_dummy = &mut Logger::new_dummy();
        // let mut git_object = objects_database::read_object(hash_commit, logger_dummy)?;
        // let commit_object = git_object
        //     .as_mut_commit()
        //     .ok_or(CommandError::InvalidCommit)?;
        // let commit_object = commit_object.to_owned();

        let (path, decompressed_data) = read_file(hash_commit, logger_dummy)?;
        let data = String::from_utf8(decompressed_data).unwrap();
        println!("{}", data);


        // let parents_hash = commit_object.get_parents();

        // let first_parent_hash = &parents_hash[0];
        // let path_to_parent = format!(
        //     ".git/objects/{}/{}",
        //     &first_parent_hash[..2],
        //     &first_parent_hash[2..]
        // );
        // self.rebuild_commits_tree(&path_to_parent, commits_map, branch.clone())?;

        // if self.all {
        //     for parent in parents_hash.iter().skip(1) {
        //         let path_to_parent = format!("../.git/objects/{}/{}", &parent[..2], &parent[2..]);
        //         if !commits_map.contains_key(&hash_commit.to_string()) {
        //             self.rebuild_commits_tree(&path_to_parent, commits_map, None)?;
        //         }
        //     }
        // }

        // let commit_with_branch = (commit_object, branch);
        // commits_map.insert(hash_commit.to_string(), commit_with_branch);
        Ok(())
    }
}

/// Agrega al vector de branches_with_their_commits todos los nombres de las ramas y el hash del commit al que apuntan
fn push_branch_hashes(
    branches_with_their_commits: &mut Vec<(String, String)>,
) -> Result<(), CommandError> {
    let path_to_heads = "../";
    let branches_hashes = local_branches(path_to_heads)?;
    for branch_hash in branches_hashes {
        branches_with_their_commits.push(branch_hash);
    }
    Ok(())
}

// /// Agrega al vector de paths_to_commits todos los paths a los últimos commits de sus respectivas ramas y el nombre de la rama
// fn get_all_branches_and_hashes(
//     path_to_heads: &str,
//     path_to_commits: &mut Vec<BranchAndHashCommit>,
// ) -> Result<(), CommandError> {
//     Ok(if let Ok(branches) = fs::read_dir(path_to_heads) {
//         for branch_file in branches {
//             let branch_file_dir = branch_file.map_err(|_| {
//                 CommandError::FileNotFound("no se pudo abrir branch en log".to_string())
//             })?;

//             let path = branch_file_dir.path();
//             let Some(branch_file_name) = path.to_str() else {
//                 return Err(CommandError::FileNotFound(format!(
//                     "No se pudo abrir branch en log"
//                 )));
//             };

//             let branch_name = branch_file_name.split('/').last();
//             let branch_name = branch_name.ok_or_else(|| {
//                 CommandError::FileNotFound("No se pudo abrir branch en log".to_string())
//             })?;
//             let path_and_branch = BranchAndHashCommit {
//                 branch_commit: branch_name.to_string(),
//                 hash_commit: get_commit_hash(&branch_file_name.to_string())?,
//             };
//             path_to_commits.push(path_and_branch);
//         }
//     } else {
//         return Err(CommandError::FileNotFound(
//             "No se pudo abrir .git/refs/heads en log".to_string(),
//         ));
//     })
// }

// /// Obtiene el hash del commit al que apunta la rama actual en la que se encuentra el usuario
// fn get_hash_and_branch_to_actual_branch() -> Result<BranchAndHashCommit, CommandError> {
//     let mut file = File::open("../.git/HEAD").map_err(|_| {
//         CommandError::FileNotFound("No se pudo abrir ../.git/HEAD en log".to_string())
//     })?;

//     let mut refs_heads = String::new();
//     file.read_to_string(&mut refs_heads).map_err(|_| {
//         CommandError::FileReadError("No se pudo leer ../.git/HEAD en log".to_string())
//     })?;

//     let Some((_, path_to_branch)) = refs_heads.split_once(' ') else {
//         return Err(CommandError::FileNotFound(
//             "No se pudo abrir ../.git/HEAD en log".to_string(),
//         ));
//     };

//     let path_to_branch = if path_to_branch.len() > 0 {
//         &path_to_branch[..path_to_branch.len() - 1]
//     } else {
//         return Err(CommandError::FileNotFound(
//             "No existe un archivo con nombre vacio en ../.git/objects considere analizarlo"
//                 .to_string(),
//         ));
//     };

//     let name_branch = path_to_branch.split('/').last();
//     let name_branch = name_branch
//         .ok_or_else(|| CommandError::FileNotFound("No se pudo abrir branch en log".to_string()))?;

//     let path_and_branch = BranchAndHashCommit {
//         branch_commit: name_branch.to_string(),
//         hash_commit: get_commit_hash(&path_to_branch.to_string())?,
//     };

//     Ok(path_and_branch)
// }

/// Obtiene el hash del commit al que apunta la rama actual en la que se encuentra el usuario
fn get_commit_hash(refs_branch_name: &String) -> Result<String, CommandError> {
    let path_to_heads = ".git/";
    let path_to_branch = format!("{}/{}", path_to_heads, refs_branch_name);

    let mut file = File::open(&path_to_branch).map_err(|_| {
        CommandError::FileNotFound(format!("No se pudo abrir {path_to_branch} en log"))
    })?;

    let mut commit_hash = String::new();
    file.read_to_string(&mut commit_hash).map_err(|_| {
        CommandError::FileReadError(format!("No se pudo leer {path_to_branch} en log"))
    })?;

    Ok(commit_hash[..commit_hash.len() - 1].to_string())
}

