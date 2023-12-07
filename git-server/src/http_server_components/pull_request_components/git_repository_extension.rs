use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use git_lib::{command_errors::CommandError, git_repository::GitRepository, join_paths};

use crate::http_server_components::http_methods::post_pull_request::PullRequest;

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
    }

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
}
