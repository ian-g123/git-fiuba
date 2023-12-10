use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use git_lib::{
    command_errors::CommandError, git_repository::GitRepository, join_paths,
    objects::commit_object::CommitObject,
};

use crate::http_server_components::http_methods::{
    pull_request::PullRequest, pull_request_state::PullRequestState,
    pull_request_update::PullRequestUpdate,
};

use super::content_type::ContentType;

pub trait GitRepositoryExtension {
    fn create_pull_request(
        &mut self,
        pull_request_info: PullRequest,
    ) -> Result<PullRequest, CommandError>;

    fn save_pull_request(
        &mut self,
        pull_request_info: &mut PullRequest,
    ) -> Result<PullRequest, CommandError>;
    fn get_last_pull_request_id(&self) -> Result<u64, CommandError>;
    fn set_last_pull_request_id(&self, pull_request_id: u64) -> Result<(), CommandError>;
    fn get_pull_requests(&mut self, state: &str) -> Result<Vec<PullRequest>, CommandError>;
    fn get_pull_request(
        &mut self,
        pull_request_id: u64,
    ) -> Result<Option<PullRequest>, CommandError>;
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

    fn get_pull_requests_path(&self) -> Result<String, CommandError>;
    fn get_pull_request_path(&self, id: u64) -> Result<Option<String>, CommandError>;
    fn get_server_files_path(&self) -> Result<String, CommandError>;
    fn update_pull_request(
        &mut self,
        id: u64,
        pull_request_info: PullRequestUpdate,
    ) -> Result<Option<PullRequest>, CommandError>;

    fn get_pull_request_format(&mut self, id: u64) -> Result<ContentType, CommandError>;
}

impl<'a> GitRepositoryExtension for GitRepository<'a> {
    fn create_pull_request(
        &mut self,
        mut pull_request: PullRequest,
    ) -> Result<PullRequest, CommandError> {
        if pull_request.id.is_some() {
            panic!("No se puede crear un pull request con un id");
        }
        if pull_request.merged.is_some() {
            panic!("No se puede crear un pull request con un merged");
        }

        let source_branch = &pull_request.source_branch;
        if !self.branch_exists(&source_branch) {
            return Err(CommandError::InvalidBranchName(source_branch.to_string()));
        };
        let target_branch = &pull_request.target_branch;
        if !self.branch_exists(&target_branch) {
            return Err(CommandError::InvalidBranchName(target_branch.to_string()));
        };
        if target_branch == source_branch {
            return Err(CommandError::NothingToCompare(format!(
                "No se puede mergear la rama {} en {}",
                target_branch, target_branch
            )));
        }
        let commits_to_merge =
            self.get_commits_to_merge(source_branch.to_string(), target_branch.to_string())?;
        if commits_to_merge.is_empty() {
            return Err(CommandError::NothingToCompare(format!(
                "{} is up-to-date with {}",
                source_branch, target_branch
            )));
        }
        pull_request.set_merged(false);
        self.save_pull_request(&mut pull_request)?;
        let has_conflicts =
            self.has_merge_conflicts(&pull_request.source_branch, &pull_request.target_branch)?;
        pull_request.has_merge_conflicts = Some(has_conflicts);
        Ok(pull_request)
    }

    fn save_pull_request(
        &mut self,
        pull_request_info: &mut PullRequest,
    ) -> Result<PullRequest, CommandError> {
        let pull_request_id = match pull_request_info.id {
            Some(id) => id,
            None => {
                let last_pull_request_id = self.get_last_pull_request_id()?;
                last_pull_request_id + 1
            }
        };
        let new_pull_request_path_str = join_paths!(
            self.get_pull_requests_path()?,
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
        pull_request_info.has_merge_conflicts = None;
        serde_json::to_writer(new_pull_request, &pull_request_info).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error escribiendo el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;
        self.set_last_pull_request_id(pull_request_id)?;
        let has_conflicts = match pull_request_info.merged {
            Some(true) => None,
            _ => Some(self.has_merge_conflicts(
                &pull_request_info.source_branch,
                &pull_request_info.target_branch,
            )?),
        };
        pull_request_info.has_merge_conflicts = has_conflicts;
        self.log(&format!("Has conflicts: {:?}", has_conflicts));
        Ok(pull_request_info.to_owned())
    }

    fn get_last_pull_request_id(&self) -> Result<u64, CommandError> {
        let path = join_paths!(self.get_server_files_path()?, "LAST_PULL_REQUEST_ID").ok_or(
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
        let path = join_paths!(self.get_server_files_path()?, "LAST_PULL_REQUEST_ID").ok_or(
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

    fn get_pull_requests(&mut self, state: &str) -> Result<Vec<PullRequest>, CommandError> {
        let pull_requests_path = self.get_pull_requests_path()?;
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
            let pull_request_file = File::open(pull_request_path).map_err(|error| {
                CommandError::FileOpenError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?;

            let mut pull_request = read_pull_request_from_file(pull_request_file)?;
            let has_conflicts =
                self.has_merge_conflicts(&pull_request.source_branch, &pull_request.target_branch)?;
            pull_request.has_merge_conflicts = Some(has_conflicts);
            match state {
                "all" => {
                    pull_requests.push(pull_request);
                }
                "open" => {
                    if pull_request.get_state() == PullRequestState::Open
                    //|| pull_request.get_state() == PullRequestState::MergeConflicts
                    {
                        pull_requests.push(pull_request);
                    }
                }
                "closed" => {
                    if pull_request.get_state() == PullRequestState::Closed {
                        pull_requests.push(pull_request);
                    }
                }
                _ => {
                    return Err(CommandError::InvalidPullRequestState(state.to_string()));
                }
            }
        }
        pull_requests.sort_unstable_by(|a, b| a.id.unwrap().cmp(&b.id.unwrap()));
        Ok(pull_requests)
    }

    fn get_pull_request(
        &mut self,
        pull_request_id: u64,
    ) -> Result<Option<PullRequest>, CommandError> {
        let pull_requests_path_str = join_paths!(
            self.get_pull_requests_path()?,
            format!("{}.json", pull_request_id)
        )
        .ok_or(CommandError::FileOpenError(
            "Error creando el path del nuevo pull request".to_string(),
        ))?;
        let pull_request_path = Path::new(&pull_requests_path_str);
        if !pull_request_path.exists() {
            return Ok(None);
        }
        let pull_request_file = File::open(pull_request_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error leyendo el directorio de pull requests: {}",
                error.to_string()
            ))
        })?;
        let mut pull_request = read_pull_request_from_file(pull_request_file)?;
        let has_conflicts = match pull_request.merged {
            Some(true) => None,
            _ => Some(
                self.has_merge_conflicts(&pull_request.source_branch, &pull_request.target_branch)?,
            ),
        };
        pull_request.has_merge_conflicts = has_conflicts;
        Ok(Some(pull_request))
    }

    fn get_pull_request_commits(
        &mut self,
        pull_request_id: u64,
    ) -> Result<Option<Vec<CommitObject>>, CommandError> {
        let Some(pull_request) = self.get_pull_request(pull_request_id)? else {
            return Ok(None);
        };
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

    fn get_pull_requests_path(&self) -> Result<String, CommandError> {
        let pull_requests_path = join_paths!(self.get_server_files_path()?, "pull_requests")
            .ok_or(CommandError::FileOpenError(
                "Error creando el path del nuevo pull request".to_string(),
            ))?;
        Ok(pull_requests_path)
    }

    fn get_server_files_path(&self) -> Result<String, CommandError> {
        let server_files_path = join_paths!(self.get_git_path(), "server_files").ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        Ok(server_files_path)
    }

    fn update_pull_request(
        &mut self,
        pull_request_id: u64,
        pull_request_info: PullRequestUpdate,
    ) -> Result<Option<PullRequest>, CommandError> {
        let Some(mut previous_pull_request) = self.get_pull_request(pull_request_id)? else {
            return Ok(None);
        };
        let source_branch = previous_pull_request.source_branch.clone();
        if let Some(target_branch) = pull_request_info.clone().target_branch {
            if !self.branch_exists(&target_branch) {
                return Err(CommandError::InvalidBranchName(target_branch.to_string()));
            };

            if target_branch == source_branch {
                return Err(CommandError::NothingToCompare(format!(
                    "No se puede mergear la rama {} en {}",
                    target_branch, target_branch
                )));
            }

            let commits_to_merge =
                self.get_commits_to_merge(source_branch.to_string(), target_branch.to_string())?;
            if commits_to_merge.is_empty() {
                return Err(CommandError::NothingToCompare(format!(
                    "{} está al día con {}",
                    source_branch, target_branch
                )));
            }
        }

        previous_pull_request.update(pull_request_info)?;

        Ok(Some(self.save_pull_request(&mut previous_pull_request)?))
    }

    fn get_pull_request_path(&self, id: u64) -> Result<Option<String>, CommandError> {
        let path_json = join_paths!(self.get_pull_requests_path()?, format!("{}.json", id)).ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        let path_plain = join_paths!(self.get_pull_requests_path()?, format!("{}.txt", id)).ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        if Path::new(&path_json).exists() {
            return Ok(Some(path_json));
        }
        if Path::new(&path_plain).exists() {
            return Ok(Some(path_plain));
        }
        Ok(None)
    }

    fn get_pull_request_format(&mut self, id: u64) -> Result<ContentType, CommandError> {
        let path_json = join_paths!(self.get_pull_requests_path()?, format!("{}.json", id)).ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        let path_plain = join_paths!(self.get_pull_requests_path()?, format!("{}.txt", id)).ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
        if Path::new(&path_json).exists() {
            return Ok(ContentType::Json);
        }
        if Path::new(&path_plain).exists() {
            return Ok(ContentType::Plain);
        }
        Err(CommandError::InvalidContentType)
    }
}

fn read_pull_request_from_file(mut pull_request_file: File) -> Result<PullRequest, CommandError> {
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
    Ok(pull_request)
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
