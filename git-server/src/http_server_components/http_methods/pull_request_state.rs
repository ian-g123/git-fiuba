use serde::{Deserialize, Serialize};

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
}
