use std::io::Read;

use git_lib::{command_errors::CommandError, utils::aux::read_string_until};
use serde::{Deserialize, Serialize};

use crate::http_server_components::http_error::HttpError;

use super::{
    pull_request_state::{default_state, PullRequestState},
    pull_request_update::PullRequestUpdate,
};

/// Representa un Pull Request.
/// id: identificador del Pull Request
/// title: título del Pull Request
/// description: descripción del Pull Request
/// source branch: rama del repositorio que se desea mergear         
/// target branch: rama del repositorio a la que se desea mergear
/// source branch --> target branch
/// state: indica el estado del Pull Request, abierto o cerrado
/// has_merge_conflicts: indica si 'source branch' tiene conflictos de merge con 'target branch'
/// merged: indica si el Pull Request ha sido mergeado o no. Todo Pull Request mergeado tiene state=closed
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
    /// Devuelve el estado del Pull Request
    pub fn get_state(&self) -> PullRequestState {
        self.state.clone()
    }

    /// Modifica el estado del Pull Request
    pub fn set_state(&mut self, state: PullRequestState) {
        self.state = state;
    }

    /// Dado un PullRequestUpdate, modifica los campos permitidos de este PullRequest.
    /// Si el Pull Request está cerrado, solo se puede modificar su estado y título.
    /// Si está mergeado, no se puede modificar.
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

    /// Devuelve true si el Pull Request está cerrado.
    pub fn is_closed(&self) -> bool {
        if let PullRequestState::Closed = self.state {
            return true;
        }
        false
    }

    /// Devuelve true si el Pull Request fue mergeado
    pub fn is_merged(&self) -> Result<bool, HttpError> {
        self.merged.ok_or(HttpError::InternalServerError(
            CommandError::PullRequestUnknownMerge,
        ))
    }

    /// Devuelve true si el Pull Request fue mergeado
    pub fn is_merged_bis(&self) -> Result<bool, CommandError> {
        self.merged.ok_or(CommandError::PullRequestUnknownMerge)
    }

    /// Modifica el valor del campo 'merged'.
    pub fn set_merged(&mut self, merged: bool) {
        self.merged = Some(merged);
    }

    /// Lee la información del PullRequest de un socket, en formato de texto plano:
    ///
    /// ---
    /// title
    /// source_branch
    /// target_branch
    /// description
    /// ---
    ///
    /// El resto de los parámetros son seteados a los valores por defecto
    pub fn from_plain(socket: &mut std::net::TcpStream, len: usize) -> Result<Self, CommandError> {
        let mut buffer = vec![0; len];
        socket.read_exact(&mut buffer).map_err(|_| {
            CommandError::InvalidHTTPRequest("Could not read pull request".to_string())
        })?;
        let mut reader = std::io::Cursor::new(buffer);
        let title = read_string_until(&mut reader, '\n')?;
        let source_branch = read_string_until(&mut reader, '\n')?;
        let target_branch = read_string_until(&mut reader, '\n')?;
        let mut description = String::new();
        reader.read_to_string(&mut description).map_err(|_| {
            CommandError::InvalidHTTPRequest("Could not read pull request".to_string())
        })?;

        Ok(Self {
            id: None,
            title,
            description,
            source_branch,
            target_branch,
            state: PullRequestState::Open,
            has_merge_conflicts: None,
            merged: None,
        })
    }

    /// Transforma el Pull Request a String, en formato de texto plano.
    ///
    /// ---
    /// id
    /// title
    /// source_branch
    /// target_branch
    /// state
    /// has_merge_conflicts
    /// merged
    /// description
    /// ---
    ///
    /// Los campos 'id', 'has_merge_conflicts' y 'merged' solo son escritos si el Option asociado es Some

    pub fn to_plain(&self) -> String {
        let mut string = String::new();
        let id = if let Some(id) = self.id {
            format!("{}\n", id)
        } else {
            "\n".to_string()
        };
        string += &id;
        string += &format!("{}\n", self.title);
        string += &format!("{}\n", self.source_branch);
        string += &format!("{}\n", self.target_branch);
        string += &format!("{}\n", self.state.to_string());

        let conflicts = if let Some(has_merge_conflicts) = self.has_merge_conflicts {
            format!("{}\n", has_merge_conflicts)
        } else {
            "\n".to_string()
        };
        string += &conflicts;
        let merged = if let Some(merged) = self.merged {
            format!("{}\n", merged)
        } else {
            "\n".to_string()
        };
        string += &merged;
        string += &format!("{}", self.description);
        string
    }
}
