use std::{f32::consts::E, io::Read};

use git_lib::{command_errors::CommandError, utils::aux::read_string_until};
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

    /// Crea un PullRequestUpdate a partir de la información leída del socket. La misma debe tener el siguiente formato:
    /// ---
    /// título
    /// target_branch
    /// estado
    /// descripción
    /// ---
    ///
    /// Un campo vacío indica que ese parámetro no se desea actualizar
    ///
    pub fn from_plain(socket: &mut std::net::TcpStream, len: usize) -> Result<Self, CommandError> {
        let mut buffer = vec![0; len];
        socket.read_exact(&mut buffer).map_err(|_| {
            CommandError::InvalidHTTPRequest("Could not read pull request".to_string())
        })?;
        let mut reader = std::io::Cursor::new(buffer);
        let title_read = read_string_until(&mut reader, '\n')?;
        let title = if title_read.is_empty() {
            None
        } else {
            Some(title_read)
        };
        let target_branch_read = read_string_until(&mut reader, '\n')?;
        let target_branch = if target_branch_read.is_empty() {
            None
        } else {
            Some(target_branch_read)
        };

        let state_read = read_string_until(&mut reader, '\n')?;
        let state = if state_read.is_empty() {
            None
        } else {
            Some(PullRequestState::from_string(&state_read)?)
        };
        let mut description_read = String::new();
        reader.read_to_string(&mut description_read).map_err(|_| {
            CommandError::InvalidHTTPRequest("Could not read pull request".to_string())
        })?;
        let description = if description_read.is_empty() {
            None
        } else {
            Some(description_read)
        };

        Ok(Self {
            title,
            description,
            target_branch,
            state,
        })
    }
}
