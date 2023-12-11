use git_lib::command_errors::CommandError;
use serde::{Deserialize, Serialize};

pub fn default_state() -> PullRequestState {
    PullRequestState::Open
}

/// Indica el estado de un PullRequest: open o closed
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestState {
    Open,
    Closed,
}

impl PullRequestState {
    /// Convierte el PullRequestState a bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            PullRequestState::Open => "open".as_bytes().to_owned(),
            PullRequestState::Closed => "closed".as_bytes().to_owned(),
        }
    }

    /// Dada una cadena, obtiene el PullRequestState correspondiente.
    /// Devuelve error si es inválida.
    pub fn from_string(state: &str) -> Result<Self, CommandError> {
        match state {
            "open" => Ok(PullRequestState::Open),
            "closed" => Ok(PullRequestState::Closed),
            s => Err(CommandError::InvalidPullRequestState(s.to_string())),
        }
    }
}

impl std::fmt::Display for PullRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PullRequestState::Open => write!(f, "open"),
            PullRequestState::Closed => write!(f, "closed"),
        }
    }
}
