use std::{collections::HashMap, fs::File, net::TcpStream, process::Command};

use git_lib::command_errors::CommandError;
use serde::Deserialize;

use crate::http_server_components::{
    http_error::HttpError,
    http_methods::{pull_request::PullRequest, pull_request_update::PullRequestUpdate},
};

use super::simplified_commit_object::SimplifiedCommitObject;

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
            _ => {
                ContentType::Json
                /* return Err(HttpError::BadRequest(format!(
                    "Content-Type not supported: {}",
                    content_type
                ))) */
            }
        };
        Ok(content_type)
    }

    /// Deserializa un pull request
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

    /// Deserializa un pull request del tipo 'update'
    pub fn deserialize_pull_request_update(
        &self,
        socket: &mut TcpStream,
        headers: &HashMap<String, String>,
    ) -> Result<PullRequestUpdate, HttpError> {
        match self {
            ContentType::Json => {
                let mut de = serde_json::Deserializer::from_reader(socket);
                PullRequestUpdate::deserialize(&mut de).map_err(|e| {
                    HttpError::BadRequest(format!("Fail to parse request body: {}", e.to_string()))
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

    pub fn seralize_pull_request(&self, pull_request: &PullRequest) -> Result<String, HttpError> {
        let response_body = match self {
            ContentType::Json => serde_json::to_string(&pull_request)
                .map_err(|_| HttpError::InternalServerError(CommandError::PullRequestToString))?,
            ContentType::Plain => pull_request.to_plain(),
        };
        Ok(response_body)
    }

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

    pub fn get_extension(&self) -> String {
        match self {
            ContentType::Json => ".json".to_string(),
            ContentType::Plain => ".txt".to_string(),
        }
    }

    pub fn pull_request_to_writer(
        &self,
        new_pull_request_file: &mut File,
        pull_request_info: &PullRequest,
    ) -> Result<(), CommandError> {
        match self {
            ContentType::Json => {
                serde_json::to_writer(new_pull_request_file, &pull_request_info).map_err(
                    |error| {
                        CommandError::FileOpenError(format!(
                            "Error escribiendo el archivo del nuevo pull request: {}",
                            error.to_string()
                        ))
                    },
                )?;
            }
            ContentType::Plain => {
                pull_request_info.write_plain(new_pull_request_file)?;
            }
        }
        Ok(())
    }

    pub fn deseralize_pull_request(
        &self,
        pull_request_content: String,
    ) -> Result<PullRequest, CommandError> {
        // PullRequest {
        let pr = match self {
            ContentType::Json => serde_json::from_str(&pull_request_content).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo el directorio de pull requests: {}",
                    error.to_string()
                ))
            })?,
            ContentType::Plain => PullRequest::from_string_plain_format(pull_request_content)?,
        };
        Ok(pr)
    }
}

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
