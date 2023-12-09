use git_lib::command_errors::CommandError;
use serde::{Deserialize, Serialize};

use crate::http_server_components::http_error::HttpError;

pub fn default_state() -> PullRequestState {
    PullRequestState::Open
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestState {
    Open,
    Closed,
}

impl PullRequestState {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            PullRequestState::Open => "open".as_bytes().to_owned(),
            PullRequestState::Closed => "closed".as_bytes().to_owned(),
        }
    }

    pub fn from_string(state: &str) -> Result<Self, CommandError> {
        match state {
            "open" => Ok(PullRequestState::Open),
            "closed" => Ok(PullRequestState::Closed),
            s => Err(CommandError::InvalidPullRequestState(s.to_string())),
        }
    }
}
