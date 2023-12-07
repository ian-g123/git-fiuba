use std::{
    collections::HashMap,
    io::{BufRead as _, BufReader, Write},
    net::TcpStream,
};

use git_lib::{
    command_errors::CommandError, git_repository::GitRepository, logger_sender::LoggerSender,
};

use crate::pull_request_components::{
    git_repository_extension::GitRepositoryExtension, post_pull_request::PostPullRequest,
};

use super::http_error::HttpError;

pub struct ServerWorker {
    path: String,
    socket: TcpStream,
    process_id: String,
    thread_id: String,
    logger_sender: LoggerSender,
}

impl ServerWorker {
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
        let buf_reader = BufReader::new(&mut self.socket);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        let mut method_uri_version = http_request[0].splitn(2, ' ');
        let method = method_uri_version
            .next()
            .ok_or(CommandError::InvalidHTTPRequest)?;
        let uri = method_uri_version
            .next()
            .ok_or(CommandError::InvalidHTTPRequest)?;
        let version = method_uri_version
            .next()
            .ok_or(CommandError::InvalidHTTPRequest)?;

        if version != "HTTP/1.1" {
            self.send_response(&400, "Bad Request", &HashMap::new(), "Invalid HTTP version")?;
            return Ok(());
        }

        let (headers, body) = get_headers_and_body(&http_request)?;

        if let Err(error) = match method {
            "POST" => self.handle_post(uri, headers, body),
            "GET" => self.handle_get(uri, headers, body),
            "PUT" => self.handle_put(uri, headers, body),
            "PATCH" => self.handle_patch(uri, headers, body),
            any => {
                self.send_response(
                    &400,
                    "Bad Request",
                    &HashMap::new(),
                    &format!("Invalid HTTP method: {}", any),
                )?;
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
        &self,
        uri: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(), HttpError> {
        let uri = uri.strip_prefix("/repos/").ok_or(HttpError::BadRequest)?;
        let mut uri_rest = uri.split('/');
        let repo_path = uri_rest.next().ok_or(HttpError::BadRequest)?;
        let object_name = uri_rest.next().ok_or(HttpError::BadRequest)?;
        if uri_rest.next().is_some() {
            return Err(HttpError::BadRequest);
        };
        if object_name != "pulls" {
            return Err(HttpError::BadRequest);
        }

        let request_info: PostPullRequest =
            serde_json::from_str(&body).map_err(|_| HttpError::BadRequest)?;

        let mut sink = std::io::sink();
        let mut repo =
            GitRepository::open(repo_path, &mut sink).map_err(|_| HttpError::NotFound)?;
        repo.create_pull_request(request_info)
            .map_err(|e| HttpError::InternalServerError(e))?;
        Ok(())
    }

    fn handle_get(
        &self,
        uri: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(), HttpError> {
        todo!()
    }

    fn handle_put(
        &self,
        uri: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(), HttpError> {
        todo!()
    }

    fn handle_patch(
        &self,
        uri: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(), HttpError> {
        todo!()
    }

    fn send_error(&mut self, error: &HttpError) -> Result<(), CommandError> {
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
    let body = while let Some(line) = peekable.next() {
        if peekable.peek().is_none() {
            return Ok((headers, line.to_string()));
        }
        let (key, value) = line
            .split_once(':')
            .ok_or(CommandError::InvalidHTTPRequest)?;
        headers.insert(key.trim().to_string(), value.trim().to_string());
    };
    Err(CommandError::InvalidHTTPRequest)
}
