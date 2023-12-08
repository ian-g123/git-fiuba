use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use git_lib::{
    command_errors::CommandError,
    git_repository::GitRepository,
    join_paths,
    logger::Logger,
    objects::{commit_object::CommitObject, git_object::GitObjectTrait},
};

use crate::http_server_components::http_methods::pull_request::PullRequest;

pub trait GitRepositoryExtension {
    fn create_pull_request(
        &mut self,
        pull_request_info: PullRequest,
    ) -> Result<PullRequest, CommandError>;

    fn save_pull_request(
        &self,
        pull_request_info: PullRequest,
    ) -> Result<PullRequest, CommandError>;
    fn get_last_pull_request_id(&self) -> Result<u64, CommandError>;
    fn set_last_pull_request_id(&self, pull_request_id: u64) -> Result<(), CommandError>;
    fn get_pull_requests(&self) -> Result<Vec<PullRequest>, CommandError>;
    fn get_pull_request(&self, pull_request_id: u64) -> Result<Option<PullRequest>, CommandError>;
    fn get_pull_request_commits(
        &mut self,
        pull_request_id: u64,
    ) -> Result<Option<Vec<CommitObject>>, CommandError>;

    fn get_commits_to_merge(
        &mut self,
        source_branch: String,
        target_branch: String,
    ) -> Result<Vec<CommitObject>, CommandError>;

    fn get_commit_from_db_and_insert(
        &mut self,
        source_commit_hash: String,
        source_commits_to_read: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError>;

    fn step_source(
        &mut self,
        source_commit_hash: String,
        source_commits_to_read: &mut HashMap<String, CommitObject>,
        read_target_commits: &mut HashMap<String, CommitObject>,
        read_source_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError>;

    fn step_target(
        &mut self,
        target_commits_to_read: &mut HashMap<String, CommitObject>,
        target_commit_hash: String,
        read_source_commits: &mut HashMap<String, CommitObject>,
        source_commits_to_read: &mut HashMap<String, CommitObject>,
        read_target_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError>;
}

impl<'a> GitRepositoryExtension for GitRepository<'a> {
    fn create_pull_request(
        &mut self,
        pull_request_info: PullRequest,
    ) -> Result<PullRequest, CommandError> {
        let source_branch = &pull_request_info.source_branch;
        if !self.branch_exists(&source_branch) {
            return Err(CommandError::InvalidBranchName(source_branch.to_string()));
        };
        let target_branch = &pull_request_info.target_branch;
        if !self.branch_exists(&target_branch) {
            return Err(CommandError::InvalidBranchName(target_branch.to_string()));
        };
        self.save_pull_request(pull_request_info)
    } //Falta: si no hay nada q mergear o target == source, no se crea

    fn save_pull_request(
        &self,
        mut pull_request_info: PullRequest,
    ) -> Result<PullRequest, CommandError> {
        let last_pull_request_id = self.get_last_pull_request_id()?;
        let pull_request_id = last_pull_request_id + 1;
        let new_pull_request_path_str = join_paths!(
            self.get_git_path(),
            "pull_requests",
            format!("{}.json", pull_request_id)
        )
        .ok_or(CommandError::FileOpenError(
            "Error creando el path del nuevo pull request".to_string(),
        ))?;
        let new_pull_request_path = Path::new(&new_pull_request_path_str);
        if let Some(parent_dir) = new_pull_request_path.parent() {
            fs::create_dir_all(parent_dir).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error creando el directorio para el nuevo pull request: {}",
                    error.to_string()
                ))
            })?;
        }
        let new_pull_request = fs::File::create(new_pull_request_path_str).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error creando el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;
        pull_request_info.id = Some(pull_request_id);
        serde_json::to_writer(new_pull_request, &pull_request_info).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error escribiendo el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;
        self.set_last_pull_request_id(pull_request_id)?;
        Ok(pull_request_info)
    }

    fn get_last_pull_request_id(&self) -> Result<u64, CommandError> {
        let path = join_paths!(self.get_git_path(), "pull_requests/LAST_PULL_REQUEST_ID").ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        if !std::path::Path::new(&path).exists() {
            return Ok(0);
        }
        let mut id_file = File::open(path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error creando el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;

        let mut content = [0; 8];

        id_file
            .read_exact(&mut content)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;

        Ok(u64::from_be_bytes(content))
    }

    fn set_last_pull_request_id(&self, pull_request_id: u64) -> Result<(), CommandError> {
        let path = join_paths!(self.get_git_path(), "pull_requests/LAST_PULL_REQUEST_ID").ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        let mut id_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path.clone())
            .map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error creando el archivo del nuevo pull request: {}",
                    error.to_string()
                ))
            })?;
        id_file
            .write_all(&pull_request_id.to_be_bytes())
            .map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error escribiendo el archivo LAST_PULL_REQUEST_ID: {}",
                    error.to_string()
                ))
            })
    }

    fn get_pull_requests(&self) -> Result<Vec<PullRequest>, CommandError> {
        let pull_requests_path = join_paths!(self.get_git_path(), "pull_requests").ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        let mut pull_requests = Vec::new();
        let Ok(pull_requests_dir) = fs::read_dir(pull_requests_path) else {
            return Ok(pull_requests);
        };

        for pull_request_file in pull_requests_dir {
            let pull_request_file = pull_request_file.map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
            let pull_request_path = pull_request_file.path();

            if pull_request_path.file_name().unwrap() == "LAST_PULL_REQUEST_ID" {
                continue;
            }
            let mut pull_request_file = File::open(pull_request_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
            let mut pull_request_content = String::new();
            pull_request_file
                .read_to_string(&mut pull_request_content)
                .map_err(|error| {
                    CommandError::FileReadError(format!(
                        "Error leyendo el directorio de pull requests: {}",
                        error.to_string()
                    ))
                })?;
            let pull_request: PullRequest =
                serde_json::from_str(&pull_request_content).map_err(|error| {
                    CommandError::FileReadError(format!(
                        "Error leyendo el directorio de pull requests: {}",
                        error.to_string()
                    ))
                })?;
            pull_requests.push(pull_request);
        }
        pull_requests.sort_unstable_by(|a, b| a.id.unwrap().cmp(&b.id.unwrap()));
        Ok(pull_requests)
    }

    fn get_pull_request(&self, pull_request_id: u64) -> Result<Option<PullRequest>, CommandError> {
        let pull_request_path_str = join_paths!(
            self.get_git_path(),
            "pull_requests",
            format!("{}.json", pull_request_id)
        )
        .ok_or(CommandError::FileOpenError(
            "Error creando el path del nuevo pull request".to_string(),
        ))?;
        let pull_request_path = Path::new(&pull_request_path_str);
        if !pull_request_path.exists() {
            return Ok(None);
        }
        let mut pull_request_file = File::open(pull_request_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error leyendo el directorio de pull requests: {}",
                error.to_string()
            ))
        })?;
        let mut pull_request_content = String::new();
        pull_request_file
            .read_to_string(&mut pull_request_content)
            .map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
        let pull_request: PullRequest =
            serde_json::from_str(&pull_request_content).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
        Ok(Some(pull_request))
    }

    fn get_pull_request_commits(
        &mut self,
        pull_request_id: u64,
    ) -> Result<Option<Vec<CommitObject>>, CommandError> {
        let pull_request_path_str = join_paths!(
            self.get_git_path(),
            "pull_requests",
            format!("{}.json", pull_request_id)
        )
        .ok_or(CommandError::FileOpenError(
            "Error creando el path del nuevo pull request".to_string(),
        ))?;
        let pull_request_path = Path::new(&pull_request_path_str);
        if !pull_request_path.exists() {
            return Ok(None);
        }
        let mut pull_request_file = File::open(pull_request_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error leyendo el directorio de pull requests: {}",
                error.to_string()
            ))
        })?;
        let mut pull_request_content = String::new();
        pull_request_file
            .read_to_string(&mut pull_request_content)
            .map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
        let pull_request: PullRequest =
            serde_json::from_str(&pull_request_content).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;
        let source_branch = pull_request.source_branch;
        let target_branch = pull_request.target_branch;
        let commits = self.get_commits_to_merge(source_branch, target_branch)?;
        Ok(Some(commits))
    }

    fn get_commits_to_merge(
        &mut self,
        source_branch: String,
        target_branch: String,
    ) -> Result<Vec<CommitObject>, CommandError> {
        let source_commit_hash = self.get_last_commit_hash_branch(&source_branch)?;
        let target_commit_hash = self.get_last_commit_hash_branch(&target_branch)?;
        let mut source_commits_to_read = HashMap::new();
        let mut target_commits_to_read = HashMap::new();
        self.get_commit_from_db_and_insert(source_commit_hash, &mut source_commits_to_read)?;
        self.get_commit_from_db_and_insert(target_commit_hash, &mut target_commits_to_read)?;
        let mut read_source_commits = HashMap::new();
        let mut read_target_commits = HashMap::new();
        loop {
            let first_source_commit = get_max(&source_commits_to_read);
            let first_target_commit = get_max(&target_commits_to_read);
            match (first_source_commit, first_target_commit) {
                (
                    Some((source_commit_hash, source_timestamp)),
                    Some((target_commit_hash, target_timestamp)),
                ) => {
                    if source_timestamp > target_timestamp {
                        self.step_source(
                            source_commit_hash,
                            &mut source_commits_to_read,
                            &mut read_target_commits,
                            &mut read_source_commits,
                        )?;
                    } else {
                        self.step_target(
                            &mut target_commits_to_read,
                            target_commit_hash,
                            &mut read_source_commits,
                            &mut source_commits_to_read,
                            &mut read_target_commits,
                        )?;
                    }
                }
                (Some((source_commit_hash, _)), None) => {
                    self.step_source(
                        source_commit_hash,
                        &mut source_commits_to_read,
                        &mut read_target_commits,
                        &mut read_source_commits,
                    )?;
                }
                (None, Some((target_commit_hash, _))) => {
                    self.step_target(
                        &mut target_commits_to_read,
                        target_commit_hash,
                        &mut read_source_commits,
                        &mut source_commits_to_read,
                        &mut read_target_commits,
                    )?;
                }
                (None, None) => {
                    break;
                }
            }
        }
        let mut commits_vec = read_source_commits
            .into_values()
            .collect::<Vec<CommitObject>>();

        commits_vec.sort_unstable_by(|a, b| b.get_timestamp().cmp(&a.get_timestamp()));

        Ok(commits_vec)
    }

    fn get_commit_from_db_and_insert(
        &mut self,
        commit_hash: String,
        commits_to_read: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError> {
        let commit = self
            .db()?
            .read_object(&commit_hash, &mut self.logger())?
            .as_mut_commit()
            .ok_or(CommandError::InvalidCommit)?
            .to_owned();
        commits_to_read.insert(commit_hash, commit);
        Ok(())
    }

    fn step_source(
        &mut self,
        source_commit_hash: String,
        source_commits_to_read: &mut HashMap<String, CommitObject>,
        read_target_commits: &mut HashMap<String, CommitObject>,
        read_source_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError> {
        let Some(source_commit) = source_commits_to_read.remove(&source_commit_hash) else {
            unreachable!()
        };
        if read_target_commits.contains_key(&source_commit_hash) {
            return Ok(());
        }
        for parent_hash in source_commit.get_parents() {
            if read_target_commits.contains_key(&parent_hash) {
                continue;
            }
            self.get_commit_from_db_and_insert(parent_hash, source_commits_to_read)?;
        }
        _ = read_source_commits.insert(source_commit_hash, source_commit);
        Ok(())
    }

    fn step_target(
        &mut self,
        target_commits_to_read: &mut HashMap<String, CommitObject>,
        target_commit_hash: String,
        read_source_commits: &mut HashMap<String, CommitObject>,
        source_commits_to_read: &mut HashMap<String, CommitObject>,
        read_target_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError> {
        let Some(target_commit) = target_commits_to_read.remove(&target_commit_hash) else {
            unreachable!()
        };
        if let Some(removed_commit) = read_source_commits.remove(&target_commit_hash) {
            remove_parents(&removed_commit, read_source_commits, source_commits_to_read);
        }
        if let Some(removed_commit) = source_commits_to_read.remove(&target_commit_hash) {
            remove_parents(&removed_commit, read_source_commits, source_commits_to_read);
        }
        for parent_hash in target_commit.get_parents() {
            self.get_commit_from_db_and_insert(parent_hash, target_commits_to_read)?;
        }
        _ = read_target_commits.insert(target_commit_hash, target_commit);
        Ok(())
    }
}

fn get_max(commits_to_read: &HashMap<String, CommitObject>) -> Option<(String, i64)> {
    let mut max = None;
    for (commit_hash, commit) in commits_to_read {
        if let Some((_, max_timestamp)) = max {
            if commit.get_timestamp() > max_timestamp {
                max = Some((commit_hash.to_string(), commit.get_timestamp()));
            }
        } else {
            max = Some((commit_hash.to_string(), commit.get_timestamp()));
        }
    }
    max
}

fn remove_parents(
    removed_commit: &CommitObject,
    read_commits: &mut HashMap<String, CommitObject>,
    commits_to_read: &mut HashMap<String, CommitObject>,
) {
    for parent_hash in removed_commit.get_parents() {
        read_commits.remove(&parent_hash);
        commits_to_read.remove(&parent_hash);
    }
}
