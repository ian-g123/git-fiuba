use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Cursor, Read, Write},
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        objects::commit_object::{
            print_for_log, read_from_for_log, sort_commits_descending_date, CommmitWithBranch,
        },
        objects_database,
    },
    logger::Logger,
};

struct BranchAndHashCommit {
    branch_commit: String,
    hash_commit: String,
}

/// Commando init
pub struct Log {
    all: bool,
}

impl Command for Log {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
        _logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "log" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        let mut commits = instance.run()?;
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

    fn run(&self) -> Result<Vec<CommmitWithBranch>, CommandError> {
        let mut path_to_commits_branch: Vec<BranchAndHashCommit> = Vec::new();
        let mut commits_map: HashMap<String, CommmitWithBranch> = HashMap::new();

        if self.all {
            let path_to_heads = "../.git/refs/heads";
            get_all_branches_and_hashes(path_to_heads, &mut path_to_commits_branch)?;
        } else {
            path_to_commits_branch.push(get_hash_and_branch_to_actual_branch()?);
        }

        for path_to_last_commit in path_to_commits_branch {
            self.rebuild_commits_tree(
                &path_to_last_commit.hash_commit,
                &mut commits_map,
                Some(path_to_last_commit.branch_commit.clone()),
            )?;
        }
        let mut commits: Vec<_> = commits_map.drain().map(|(_, v)| v).collect();

        sort_commits_descending_date(&mut commits);

        Ok(commits)
    }

    /// Reconstruye el arbol de commits que le preceden a partir de un commit
    fn rebuild_commits_tree(
        &self,
        path_to_commit: &String,
        commits_map: &mut HashMap<String, CommmitWithBranch>,
        branch: Option<String>,
    ) -> Result<(), CommandError> {
        if commits_map.contains_key(path_to_commit) {
            return Ok(());
        }

        let mut logger_dummy = Logger::new_dummy();

        let (_, decompressed_data) =
            objects_database::read_file(path_to_commit, &mut logger_dummy)?;
        let output_str = String::from_utf8(decompressed_data).map_err(|error| {
            logger_dummy.log("Error conviertiendo a utf8 el contenido en log");
            CommandError::FileReadError(error.to_string())
        })?;

        let mut deflated_file_reader = Cursor::new(output_str);
        let commit_object = read_from_for_log(&mut deflated_file_reader, &mut logger_dummy)?;

        let parents = commit_object.get_parents();

        let first_parent = &parents[0];
        let path_to_parent = format!(
            "../.git/objects/{}/{}",
            &first_parent[..2],
            &first_parent[2..]
        );
        self.rebuild_commits_tree(&path_to_parent, commits_map, branch.clone())?;

        if !self.all {
            for i in 1..parents.len() {
                let parent = &parents[i];
                let path_to_parent = format!("../.git/objects/{}/{}", &parent[..2], &parent[2..]);
                if !commits_map.contains_key(&path_to_commit.to_string()) {
                    self.rebuild_commits_tree(&path_to_parent, commits_map, None)?;
                }
            }
        }
        let commit_with_branch = CommmitWithBranch::new(commit_object, branch);
        commits_map.insert(path_to_commit.to_string(), commit_with_branch);
        Ok(())
    }
}

/// Agrega al vector de paths_to_commits todos los paths a los últimos commits de sus respectivas ramas y el nombre de la rama
fn get_all_branches_and_hashes(
    path_to_heads: &str,
    path_to_commits: &mut Vec<BranchAndHashCommit>,
) -> Result<(), CommandError> {
    Ok(if let Ok(branches) = fs::read_dir(path_to_heads) {
        for branch_file in branches {
            let branch_file_dir = branch_file.map_err(|_| {
                CommandError::FileNotFound("no se pudo abrir branch en log".to_string())
            })?;

            let path = branch_file_dir.path();
            let Some(branch_file_name) = path.to_str() else {
                return Err(CommandError::FileNotFound(format!(
                    "No se pudo abrir branch en log"
                )));
            };

            let branch_name = branch_file_name.split('/').last();
            let branch_name = branch_name.ok_or_else(|| {
                CommandError::FileNotFound("No se pudo abrir branch en log".to_string())
            })?;
            let path_and_branch = BranchAndHashCommit {
                branch_commit: branch_name.to_string(),
                hash_commit: get_commit_hash(&branch_file_name.to_string())?,
            };
            path_to_commits.push(path_and_branch);
        }
    } else {
        return Err(CommandError::FileNotFound(
            "No se pudo abrir .git/refs/heads en log".to_string(),
        ));
    })
}

/// Obtiene el hash del commit al que apunta la rama actual en la que se encuentra el usuario
fn get_hash_and_branch_to_actual_branch() -> Result<BranchAndHashCommit, CommandError> {
    let mut file = File::open("../.git/HEAD").map_err(|_| {
        CommandError::FileNotFound("No se pudo abrir ../.git/HEAD en log".to_string())
    })?;

    let mut refs_heads = String::new();
    file.read_to_string(&mut refs_heads).map_err(|_| {
        CommandError::FileReadError("No se pudo leer ../.git/HEAD en log".to_string())
    })?;

    let Some((_, path_to_branch)) = refs_heads.split_once(' ') else {
        return Err(CommandError::FileNotFound(
            "No se pudo abrir ../.git/HEAD en log".to_string(),
        ));
    };

    let path_to_branch = if path_to_branch.len() > 0 {
        &path_to_branch[..path_to_branch.len() - 1]
    } else {
        return Err(CommandError::FileNotFound(
            "No existe un archivo con nombre vacio en ../.git/objects considere analizarlo"
                .to_string(),
        ));
    };

    let name_branch = path_to_branch.split('/').last();
    let name_branch = name_branch
        .ok_or_else(|| CommandError::FileNotFound("No se pudo abrir branch en log".to_string()))?;

    let path_and_branch = BranchAndHashCommit {
        branch_commit: name_branch.to_string(),
        hash_commit: get_commit_hash(&path_to_branch.to_string())?,
    };

    Ok(path_and_branch)
}

/// Obtiene el hash del commit al que apunta la rama actual en la que se encuentra el usuario
fn get_commit_hash(path_to_branch: &String) -> Result<String, CommandError> {
    let branch = format!("../.git/{}", path_to_branch);

    let mut file = File::open(&branch).map_err(|_| {
        CommandError::FileNotFound(format!("No se pudo abrir .git/refs/hea{branch} en log"))
    })?;

    let mut commit_hash = String::new();
    file.read_to_string(&mut commit_hash)
        .map_err(|_| CommandError::FileReadError(format!("No se pudo leer {branch} en log")))?;

    Ok(commit_hash[..commit_hash.len() - 1].to_string())
}
