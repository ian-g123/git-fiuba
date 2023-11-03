use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Write},
};

use crate::commands::command::Command;
use git_lib::{
    command_errors::CommandError,
    git_repository::{self, get_head_ref, local_branches},
    logger::Logger,
    objects::commit_object::{print_for_log, sort_commits_descending_date, CommitObject},
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
        let mut commits = instance.run(output)?;
        print_for_log(output, &mut commits)?;
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

    // pub fn run_for_graph() -> Result<Vec<(CommitObject, Option<String>)>, CommandError> {
    //     let log = Self::new(["--all".to_string()].as_slice()).unwrap();
    //     let commits: Vec<(CommitObject, Option<String>)> = log.run().unwrap();

    //     Ok(commits)
    // }

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

    fn run(
        &self,
        output: &mut dyn Write,
    ) -> Result<Vec<(CommitObject, Option<String>)>, CommandError> {
        let mut branches_with_their_last_hash: Vec<(String, String)> = Vec::new(); // Vec<(branch, hashes)>
        let mut commits_map: HashMap<String, (CommitObject, Option<String>)> = HashMap::new(); // HashMap<hash, (commit, branch)>

        if self.all {
            push_branch_hashes(&mut branches_with_their_last_hash)?;
        } else {
            let current_branch = get_head_ref()?;
            let hash_commit = get_commit_hash(&current_branch)?;
            branches_with_their_last_hash.push((current_branch, hash_commit));
        }

        let mut repo = git_repository::GitRepository::open("", output)?;

        for branch_with_commit in branches_with_their_last_hash {
            repo.rebuild_commits_tree(
                &branch_with_commit.1,
                &mut commits_map,
                Some(branch_with_commit.0),
                self.all,
            )?;
        }

        let mut commits: Vec<_> = commits_map.drain().map(|(_, v)| v).collect();

        sort_commits_descending_date(&mut commits);

        Ok(commits)
    }
}

/// Agrega al vector de branches_with_their_commits todos los nombres de las ramas y el hash del commit al que apuntan
fn push_branch_hashes(
    branches_with_their_commits: &mut Vec<(String, String)>,
) -> Result<(), CommandError> {
    let branches_hashes = local_branches(".")?;
    for branch_hash in branches_hashes {
        let branch_hash = (
            branch_hash.0,
            branch_hash.1[..branch_hash.1.len() - 1].to_string(),
        );
        branches_with_their_commits.push(branch_hash);
    }
    Ok(())
}

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
