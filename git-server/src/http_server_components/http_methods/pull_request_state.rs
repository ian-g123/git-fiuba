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
