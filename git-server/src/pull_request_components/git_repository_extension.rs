use std::{
    fs::{self, File},
    io::{Read, Write},
};

use git_lib::{command_errors::CommandError, git_repository::GitRepository, join_paths};

use super::post_pull_request::PostPullRequest;

pub trait GitRepositoryExtension {
    fn create_pull_request(
        &mut self,
        pull_request_info: PostPullRequest,
    ) -> Result<u64, CommandError>;

    fn save_pull_request(&self, pull_request_info: PostPullRequest) -> Result<u64, CommandError>;
    fn get_last_pull_request_id(&self) -> Result<u64, CommandError>;
    fn set_last_pull_request_id(&self, pull_request_id: u64) -> Result<(), CommandError>;
}

impl<'a> GitRepositoryExtension for GitRepository<'a> {
    fn create_pull_request(
        &mut self,
        pull_request_info: PostPullRequest,
    ) -> Result<u64, CommandError> {
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

    fn save_pull_request(&self, pull_request_info: PostPullRequest) -> Result<u64, CommandError> {
        let last_pull_request_id = self.get_last_pull_request_id()?;
        let pull_request_id = last_pull_request_id + 1;
        let new_pull_request_path = join_paths!(
            self.get_git_path(),
            "pull_requests",
            format!("{}.json", pull_request_id)
        )
        .ok_or(CommandError::FileOpenError(
            "Error creando el path del nuevo pull request".to_string(),
        ))?;
        let new_pull_request = fs::File::create(new_pull_request_path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error creando el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;
        serde_json::to_writer(new_pull_request, &pull_request_info).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error escribiendo el archivo del nuevo pull request: {}",
                error.to_string()
            ))
        })?;
        self.set_last_pull_request_id(pull_request_id)?;
        Ok(pull_request_id)
    }

    fn get_last_pull_request_id(&self) -> Result<u64, CommandError> {
        let path = join_paths!(self.get_git_path(), "pull_requests/LAST_PULL_REQUEST_ID").ok_or(
            CommandError::FileOpenError("Error creando el path del nuevo pull request".to_string()),
        )?;
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
        let mut id_file = fs::File::create(path).map_err(|error| {
            CommandError::FileOpenError(format!(
                "Error abriendo el archivo LAST_PULL_REQUEST_ID: {}",
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
}
