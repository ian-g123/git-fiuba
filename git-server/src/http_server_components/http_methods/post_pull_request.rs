use serde::{Deserialize, Serialize};

use super::pull_request_status::{default_status, PullRequestStatus};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: Option<u64>,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    #[serde(default = "default_status")]
    status: PullRequestStatus,
}
