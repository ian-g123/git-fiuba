use serde::{Deserialize, Serialize};

pub fn default_status() -> PullRequestStatus {
    PullRequestStatus::Open
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestStatus {
    Open,
    Closed,
}
