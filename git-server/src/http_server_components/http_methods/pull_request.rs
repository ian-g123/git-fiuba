use std::{
    io::{Read, Write},
    process::Command,
    str::FromStr,
};

use git_lib::{
    command_errors::CommandError, git_repository::next_line, utils::aux::read_string_until,
};
use serde::{Deserialize, Serialize};

use crate::http_server_components::http_error::HttpError;

use super::{
    from_plain::FromPlain,
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

    /// Seraliza la información del PullRequest, en formato de texto plano:
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
    pub fn write_plain(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        /* let mut buffer = Vec::new();
        if let Some(id) = self.id {
            buffer.extend_from_slice(&id.to_be_bytes());
        }
        buffer.push(b'\n');
        buffer.extend_from_slice(self.title.as_bytes());
        buffer.push(b'\n');
        buffer.extend_from_slice(self.source_branch.as_bytes());
        buffer.push(b'\n');
        buffer.extend_from_slice(self.target_branch.as_bytes());
        buffer.push(b'\n');
        buffer.extend_from_slice(&self.state.as_bytes());
        buffer.push(b'\n');
        match self.has_merge_conflicts {
            Some(true) => buffer.extend_from_slice("true".as_bytes()),
            Some(false) => buffer.extend_from_slice("false".as_bytes()),
            None => {}
        }
        buffer.push(b'\n');
        match self.merged {
            Some(true) => buffer.extend_from_slice("true".as_bytes()),
            Some(false) => buffer.extend_from_slice("false".as_bytes()),
            None => {}
        }
        buffer.push(b'\n');
        buffer.extend_from_slice(self.description.as_bytes()); */
        let content = self.to_string_plain_format();
        output.write_all(&content.as_bytes()).map_err(|_| {
            CommandError::InvalidHTTPRequest("Could not write pull request".to_string())
        })
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

    pub fn to_string_plain_format(&self) -> String {
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

    pub fn from_string_plain_format(content: String) -> Result<Self, CommandError> {
        let mut lines = content.lines();

        let id_read: String = read_next_line(&mut lines)?;
        let id = if id_read.is_empty() {
            None
        } else {
            Some(u64::from_str(&id_read).map_err(|e| CommandError::CastingError)?)
        };
        let title = read_next_line(&mut lines)?;
        let source_branch = read_next_line(&mut lines)?;
        let target_branch = read_next_line(&mut lines)?;
        let state = PullRequestState::from_string(&read_next_line(&mut lines)?)?;
        let has_merge_conflicts_read = read_next_line(&mut lines)?;
        let has_merge_conflicts = if has_merge_conflicts_read.is_empty() {
            None
        } else {
            match has_merge_conflicts_read.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => return Err(CommandError::PullRequestFromString),
            }
        };
        let merged_read = read_next_line(&mut lines)?;
        let merged: Option<bool> = if merged_read.is_empty() {
            None
        } else {
            match merged_read.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => return Err(CommandError::PullRequestFromString),
            }
        };
        let description = read_next_line(&mut lines)?;
        Ok(Self {
            id,
            title,
            description,
            source_branch,
            target_branch,
            state,
            has_merge_conflicts,
            merged,
        })
    }
}

/* impl<'a> FromPlain<'a> for PullRequest {
    fn from_plain(socket: &mut std::net::TcpStream, len: usize) -> Result<Self, CommandError> {
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
} */

/// Devuelve la próxima línea del iterador.
pub fn read_next_line(lines: &mut std::str::Lines<'_>) -> Result<String, CommandError> {
    let Some(line) = lines.next() else {
        return Err(CommandError::InvalidHTTPRequest(
            "No se pudo leer el pull request a partir de su contenido".to_string(),
        ));
    };
    Ok(line.to_string())
}
