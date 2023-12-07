use std::{collections::HashMap, io::Write, net::TcpStream};

use super::{
    http_methods::post_pull_request::PostPullRequest,
    pull_request_components::git_repository_extension::GitRepositoryExtension,
};
use git_lib::{
    command_errors::CommandError, git_repository::GitRepository, join_paths,
    logger_sender::LoggerSender, utils::aux::read_string_until,
};
use serde::Deserialize;

use super::http_error::HttpError;

pub struct ServerWorker {
    path: String,
    socket: TcpStream,
    process_id: String,
    thread_id: String,
    logger_sender: LoggerSender,
}

impl<'a> ServerWorker {
    pub fn new(path: String, stream: TcpStream, logger_sender: LoggerSender) -> Self {
        let process_id = format!("{:?}", std::process::id());
        let thread_id = format!("{:?}", std::thread::current().id());
        Self {
            path,
            socket: stream,
            process_id,
            thread_id,
            logger_sender,
        }
    }

    fn log(&mut self, message: &str) {
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.logger_sender.log(&format!(
            "[{}:{}] {}: {}",
            self.process_id, self.thread_id, time, message
        ));
    }

    pub fn handle_connection(&mut self) {
        self.log("New connection");
        match self.handle_connection_priv() {
            Ok(_) => self.log("Connection handled successfully"),
            Err(error) => {
                self.log(&format!("❌ Error: {}", error));
                eprintln!("{error}")
            }
        }
    }

    fn handle_connection_priv(&mut self) -> Result<(), CommandError> {
        let mut headers = HashMap::<String, String>::new();
        let first_line = read_string_until(&mut self.socket, '\n')?
            .trim()
            .to_string();
        self.log(&format!("⏬: {}", first_line));
        loop {
            let line = read_string_until(&mut self.socket, '\n')?
                .trim()
                .to_string();
            if line.is_empty() {
                break;
            }
            let (key, value) = line
                .split_once(':')
                .ok_or(CommandError::InvalidHTTPRequest(format!(
                    "Invalid header line: {}",
                    line
                )))?;
            self.log(&format!("⏬: {} {}", key, value));
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }

        let mut method_uri_version = first_line.split(' ');
        let Some(method) = method_uri_version.next() else {
            self.log(&format!("Invalid HTTP request: {:?}", first_line));
            self.send_error(&HttpError::BadRequest("Fail to parse method".to_string()))?;
            return Ok(());
        };
        let Some(uri) = method_uri_version.next() else {
            self.log(&format!("Invalid HTTP request: {:?}", first_line));
            self.send_error(&HttpError::BadRequest("Fail to parse uir".to_string()))?;
            return Ok(());
        };
        let Some(version) = method_uri_version.next() else {
            self.log(&format!("Invalid HTTP request: {:?}", first_line));
            self.send_error(&HttpError::BadRequest("Fail to parse version".to_string()))?;
            return Ok(());
        };

        if version != "HTTP/1.1" {
            self.log(&format!("Invalid HTTP request: {:?}", first_line));
            self.send_error(&HttpError::BadRequest(
                "HTTP Version not supported".to_string(),
            ))?;
            return Ok(());
        }

        // let (headers, body) = get_headers_and_body(&http_request)?;

        if let Err(error) = match method {
            "POST" => self.handle_post(uri, headers),
            "GET" => self.handle_get(uri, headers),
            "PUT" => self.handle_put(uri, headers),
            "PATCH" => self.handle_patch(uri, headers),
            any => {
                self.log(&format!("Invalid HTTP request: {:?}", first_line));
                self.send_error(&HttpError::BadRequest(format!(
                    "Invalid HTTP method: {}",
                    any
                )))?;

                return Ok(());
            }
        } {
            self.send_error(&error)?;
            return Ok(());
        }
        Ok(())
    }

    fn send_response(
        &mut self,
        code: &u16,
        reason: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<(), CommandError> {
        self.log(&format!("⏫ Response: {} {}\n{}", code, reason, body));
        let mut response = format!("HTTP/1.1 {} {}\r\n", code, reason);
        for (key, value) in headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        response.push_str("\r\n");
        response.push_str(body);
        self.socket
            .write_all(response.as_bytes())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn handle_post(
        &mut self,
        uri: &str,
        _headers: HashMap<String, String>,
    ) -> Result<(), HttpError> {
        self.log("Handling POST request");
        let uri = uri
            .strip_prefix("/repos/")
            .ok_or(HttpError::BadRequest("Resources not available".to_string()))?;
        let mut uri_rest = uri.split('/');
        let repo_path = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("No repo specified".to_string()))?;
        let object_name = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should end with pulls".to_string()))?;
        if uri_rest.next().is_some() {
            return Err(HttpError::BadRequest("Should end with pulls".to_string()));
        };
        if object_name != "pulls" {
            return Err(HttpError::BadRequest("Should end with pulls".to_string()));
        }

        let mut de = serde_json::Deserializer::from_reader(&mut self.socket);
        let request_info = PostPullRequest::deserialize(&mut de).unwrap();
        self.log(&format!("Request info: {:?}", request_info));
        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let saved_pull_request = repo
            .create_pull_request(request_info)
            .map_err(|e| HttpError::InternalServerError(e))?;
        let response_body = serde_json::to_string(&saved_pull_request).unwrap();
        self.send_response(&200, "OK", &HashMap::new(), &response_body)
            .map_err(|e| HttpError::InternalServerError(e))?;

        Ok(())
    }

    fn get_repo(
        &self,
        repo_path: &str,
        sink: &'a mut std::io::Sink,
    ) -> Result<GitRepository<'a>, HttpError> {
        let complete_repo_path = join_paths!(&self.path, repo_path)
            .ok_or(HttpError::InternalServerError(CommandError::JoiningPaths))?;
        let repo =
            GitRepository::open(&complete_repo_path, sink).map_err(|_| HttpError::NotFound)?;
        Ok(repo)
    }

    fn handle_get(&self, uri: &str, headers: HashMap<String, String>) -> Result<(), HttpError> {
        todo!()
    }

    fn handle_put(&self, uri: &str, headers: HashMap<String, String>) -> Result<(), HttpError> {
        todo!()
    }

    fn handle_patch(&self, uri: &str, headers: HashMap<String, String>) -> Result<(), HttpError> {
        todo!()
    }

    fn send_error(&mut self, error: &HttpError) -> Result<(), CommandError> {
        self.log(&format!("Error ❌: {}", error));
        self.send_response(
            &error.code(),
            &error.message(),
            &HashMap::new(),
            &error.body(),
        )?;
        Ok(())
    }
}

fn get_headers_and_body(
    http_request: &Vec<String>,
) -> Result<(HashMap<String, String>, String), CommandError> {
    let mut headers = HashMap::<String, String>::new();
    let mut peekable = http_request.iter().skip(1).peekable();
    while let Some(line) = peekable.next() {
        if peekable.peek().is_none() {
            return Ok((headers, line.to_string()));
        }
        let (key, value) = line
            .split_once(':')
            .ok_or(CommandError::InvalidHTTPRequest(format!(
                "Invalid header line: {}",
                line
            )))?;
        headers.insert(key.trim().to_string(), value.trim().to_string());
    }
    Err(CommandError::InvalidHTTPRequest(
        "No body found".to_string(),
    ))
}
