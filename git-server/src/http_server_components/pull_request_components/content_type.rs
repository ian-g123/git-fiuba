use std::{collections::HashMap, net::TcpStream, process::Command};

use git_lib::command_errors::CommandError;
use serde::Deserialize;

use crate::http_server_components::{
    http_error::HttpError,
    http_methods::{pull_request::PullRequest, pull_request_update::PullRequestUpdate},
};

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
            "text/json" => ContentType::Json,
            "text/plain" => ContentType::Plain,
            _ => {
                return Err(HttpError::BadRequest(format!(
                    "Content-Type not supported: {}",
                    content_type
                )))
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
                PullRequest::deserialize(&mut de).map_err(|e| {
                    HttpError::BadRequest(
                        CommandError::InvalidHTTPRequest(
                            "could not deserialize pull request".to_string(),
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

    pub fn pull_request_to_string(&self, pull_request: &PullRequest) -> Result<String, HttpError> {
        let response_body = match self {
            ContentType::Json => serde_json::to_string(&pull_request)
                .map_err(|_| HttpError::InternalServerError(CommandError::PullRequestToString))?,
            ContentType::Plain => pull_request.to_string_plain_format(),
        };
        Ok(response_body)
    }

    pub fn pull_requests_to_string(
        &self,
        pull_requests: &Vec<PullRequest>,
    ) -> Result<String, HttpError> {
        let mut string = String::new();
        for pr in pull_requests {
            string += &format!("{}\n", self.pull_request_to_string(pr)?);
        }

        Ok(string.trim().to_string())
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
