use git_lib::command_errors::CommandError;
use serde::{Deserialize, Serialize};

use crate::http_server_components::http_error::HttpError;

use super::{
    pull_request_state::{default_state, PullRequestState},
    pull_request_update::PullRequestUpdate,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PullRequest {
    pub id: Option<u64>,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    #[serde(default = "default_state")]
    state: PullRequestState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_merge_conflicts: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged: Option<bool>,
}

impl PullRequest {
    pub fn get_state(&self) -> PullRequestState {
        self.state.clone()
    }

    pub fn set_state(&mut self, state: PullRequestState) {
        self.state = state;
    }

    pub fn update(&mut self, pull_request_info: PullRequestUpdate) -> Result<(), CommandError> {
        if self.is_merged_bis()? {
            return Err(CommandError::PullRequestMerged);
        }
        if let Some(description) = pull_request_info.clone().description {
            if self.is_closed() {
                return Err(CommandError::PullRequestClosed("description".to_string()));
            }
            self.description = description;
        }
        if let Some(target_branch) = pull_request_info.clone().target_branch {
            if self.is_closed() {
                return Err(CommandError::PullRequestClosed("target branch".to_string()));
            }
            self.target_branch = target_branch;
        }

        if let Some(title) = pull_request_info.clone().title {
            self.title = title;
        }
        if let Some(state) = pull_request_info.get_state() {
            self.state = state;
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        if let PullRequestState::Closed = self.state {
            return true;
        }
        false
    }

    pub fn is_merged(&self) -> Result<bool, HttpError> {
        self.merged.ok_or(HttpError::InternalServerError(
            CommandError::PullRequestUnknownMerge,
        ))
    }

    pub fn is_merged_bis(&self) -> Result<bool, CommandError> {
        self.merged.ok_or(CommandError::PullRequestUnknownMerge)
    }

    pub fn set_merged(&mut self, merged: bool) {
        self.merged = Some(merged);
    }
}
