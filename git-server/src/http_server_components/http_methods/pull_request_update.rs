use serde::{Deserialize, Serialize};

use super::pull_request_state::PullRequestState;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PullRequestUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_branch: Option<String>,
    state: Option<PullRequestState>,
}

impl PullRequestUpdate {
    pub fn get_state(&self) -> Option<PullRequestState> {
        self.state.clone()
    }
}
