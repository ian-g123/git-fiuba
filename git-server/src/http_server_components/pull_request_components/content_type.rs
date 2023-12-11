use std::{collections::HashMap, net::TcpStream};

use git_lib::command_errors::CommandError;
use serde::Deserialize;

use crate::http_server_components::{
    http_error::HttpError,
    http_methods::{pull_request::PullRequest, pull_request_update::PullRequestUpdate},
};

use super::simplified_commit_object::SimplifiedCommitObject;

/// Maneja la serialización y deserialización de Pull Requests.
#[derive(Clone)]
pub enum ContentType {
    Json,
    Plain,
}

impl ContentType {
    /// Crea un nuevo ContentType. Si el mismo es inválido, devuelve error.
    pub fn new(headers: &HashMap<String, String>) -> Result<Self, HttpError> {
        let content_type = match headers.get("Content-Type") {
            Some(content_type) => content_type,
            None => "text/json",
        };
        let content_type = match content_type {
            "text/plain" => ContentType::Plain,
            _ => ContentType::Json,
        };
        Ok(content_type)
    }

    /// Deserializa un pull request según el ContentType.
    pub fn deserialize_pull_request(
        &self,
        socket: &mut TcpStream,
        headers: &HashMap<String, String>,
    ) -> Result<PullRequest, HttpError> {
        match self {
            ContentType::Json => {
                let mut de = serde_json::Deserializer::from_reader(socket);
                PullRequest::deserialize(&mut de).map_err(|_: serde_json::Error| {
                    HttpError::BadRequest(
                        CommandError::InvalidHTTPRequest(
                            "failed to deserialize pull request".to_string(),
                        )
                        .to_string(),
                    )
                })
            }
            ContentType::Plain => {
                let len = get_len(headers)?;
                PullRequest::from_plain(socket, len).map_err(|e| match e {
                    CommandError::InvalidHTTPRequest(message) => HttpError::BadRequest(message),
                    e => HttpError::BadRequest(
                        CommandError::InvalidHTTPRequest(e.to_string()).to_string(),
                    ),
                })
            }
        }
    }

    /// Deserializa un pull request del tipo 'update' según el ContentType
    pub fn deserialize_pull_request_update(
        &self,
        socket: &mut TcpStream,
        headers: &HashMap<String, String>,
    ) -> Result<PullRequestUpdate, HttpError> {
        match self {
            ContentType::Json => {
                let mut de = serde_json::Deserializer::from_reader(socket);
                PullRequestUpdate::deserialize(&mut de).map_err(|e| {
                    HttpError::BadRequest(format!("Fail to parse request body: {}", e))
                })
            }
            ContentType::Plain => {
                let len = get_len(headers)?;
                PullRequestUpdate::from_plain(socket, len).map_err(|e| match e {
                    CommandError::InvalidHTTPRequest(message) => HttpError::BadRequest(message),
                    e => HttpError::BadRequest(
                        CommandError::InvalidHTTPRequest(e.to_string()).to_string(),
                    ),
                })
            }
        }
    }

    /// Serializa un Pull Request según el ContentType
    pub fn serialize_pull_request(&self, pull_request: &PullRequest) -> Result<String, HttpError> {
        let response_body = match self {
            ContentType::Json => serde_json::to_string(&pull_request)
                .map_err(|_| HttpError::InternalServerError(CommandError::PullRequestToString))?,
            ContentType::Plain => pull_request.to_plain(),
        };
        Ok(response_body)
    }

    /// Convierte un vector de Pull Requests a String, según el ContentType.
    pub fn pull_requests_to_string(
        &self,
        pull_requests: &Vec<PullRequest>,
    ) -> Result<String, HttpError> {
        let response_body = match self {
            ContentType::Json => serde_json::to_string(&pull_requests)
                .map_err(|_| HttpError::InternalServerError(CommandError::PullRequestToString))?,
            ContentType::Plain => {
                let mut string = String::new();
                for pr in pull_requests {
                    string += &format!("{}\n", pr.to_plain());
                }
                string
            }
        };
        Ok(response_body)
    }

    /// Recibe un vector de SimplifiedCommitObject y lo convierte a String, según el ContentType
    pub fn commits_to_string(
        &self,
        commits: &Vec<SimplifiedCommitObject>,
    ) -> Result<String, HttpError> {
        let response_body = match self {
            ContentType::Json => serde_json::to_string(&commits)
                .map_err(|_| HttpError::InternalServerError(CommandError::PullRequestToString))?,
            ContentType::Plain => {
                let mut string = String::new();
                for commit in commits {
                    string += &format!("{}\n", commit.to_string_plain_format());
                }
                string
            }
        };
        Ok(response_body)
    }
}

/// Obtiene la longitud (Content-Length) del Body en el Content-Type especificado.
fn get_len(headers: &HashMap<String, String>) -> Result<usize, HttpError> {
    let len = headers
        .get("Content-Length")
        .ok_or(HttpError::BadRequest(
            "Content-Length header not found".to_string(),
        ))?
        .parse::<usize>()
        .map_err(|_| HttpError::BadRequest("Content-Length header is not a number".to_string()))?;
    Ok(len)
}
