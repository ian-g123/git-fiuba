use std::{collections::HashMap, fmt::format, io::Write, net::TcpStream, str::FromStr};

use super::{
    http_methods::{
        pull_request::PullRequest, pull_request_state::PullRequestState,
        pull_request_update::PullRequestUpdate,
    },
    pull_request_components::{
        git_repository_extension::GitRepositoryExtension,
        simplified_commit_object::SimplifiedCommitObject,
    },
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
        self.log(&format!("⬇️: {}", first_line));
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
            self.log(&format!("⬇️: {} {}", key, value));
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
            self.send_error(&HttpError::BadRequest("Fail to parse uri".to_string()))?;
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
        let request_info = PullRequest::deserialize(&mut de).unwrap();
        self.log(&format!("Request info: {:?}", request_info));

        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let saved_pull_request = repo
            .create_pull_request(request_info)
            .map_err(|e| match e {
                CommandError::NothingToCompare(e) => {
                    HttpError::Forbidden(CommandError::NothingToCompare(e).to_string())
                }
                CommandError::InvalidBranchName(branch) => {
                    HttpError::Forbidden(format!("{} no es una rama existente", branch))
                }

                _ => HttpError::InternalServerError(e),
            })?;
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

    fn handle_get(
        &mut self,
        uri_and_variables: &str,
        _headers: HashMap<String, String>,
    ) -> Result<(), HttpError> {
        self.log("Handling GET request");
        let (uri, variables) = match uri_and_variables.split_once('?') {
            Some((uri, variables)) => (uri, variables),
            None => (uri_and_variables, ""),
        };
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
        let id_opt = uri_rest.next();
        let commits_opt = uri_rest.next();
        if object_name != "pulls" {
            return Err(HttpError::BadRequest("Should end with pulls".to_string()));
        }

        match (id_opt, commits_opt) {
            (None, None) => self.handle_get_pull_requests(repo_path, variables),
            (Some(id), None) => self.handle_get_pull_request(repo_path, u64::from_str(id).unwrap()),
            (Some(id), Some("commits")) => {
                self.handle_get_pull_request_commits(repo_path, u64::from_str(id).unwrap())
            }
            _ => Err(HttpError::BadRequest("Invalid uri".to_string())),
        }
    }

    fn handle_put(
        &mut self,
        uri: &str,
        _headers: HashMap<String, String>,
    ) -> Result<(), HttpError> {
        self.log("Handling PUT request");
        let uri = uri
            .strip_prefix("/repos/")
            .ok_or(HttpError::BadRequest("Resources not available".to_string()))?;
        let mut uri_rest = uri.split('/');
        let repo_path = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("No repo specified".to_string()))?;
        let pulls_name = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should end with pulls".to_string()))?;
        let id = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should have an id".to_string()))?;
        let merge_name = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should end with 'merge'".to_string()))?;

        if pulls_name != "pulls" {
            return Err(HttpError::BadRequest("Should end with pulls".to_string()));
        };

        if merge_name != "merge" {
            return Err(HttpError::BadRequest("Should end with merge".to_string()));
        };

        self.handle_put_pull_request(repo_path, u64::from_str(id).unwrap())?;
        Ok(())
    }

    //PATCH /repos/{repo}/pulls/{pull_number}
    fn handle_patch(
        &mut self,
        uri: &str,
        _headers: HashMap<String, String>,
    ) -> Result<(), HttpError> {
        self.log("Handling PATCH request");
        self.log(&format!("URI: {}", uri));
        let uri = uri
            .strip_prefix("/repos/")
            .ok_or(HttpError::BadRequest("Resources not available".to_string()))?;
        let mut uri_rest = uri.split('/');
        let repo_path = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("No repo specified".to_string()))?;
        let pulls_name = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should end with pulls".to_string()))?;
        let id = uri_rest
            .next()
            .ok_or(HttpError::BadRequest("Should have an id".to_string()))?;

        if pulls_name != "pulls" {
            return Err(HttpError::BadRequest("Should end with pulls".to_string()));
        };

        let mut de = serde_json::Deserializer::from_reader(&mut self.socket);
        let request_info = PullRequestUpdate::deserialize(&mut de).map_err(|e| {
            HttpError::BadRequest(format!("Fail to parse request body: {}", e.to_string()))
        })?;

        self.log(&format!("Request info: {:?}", request_info));
        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let saved_pull_request = repo
            .update_pull_request(u64::from_str(id).unwrap(), request_info)
            .map_err(|e| match e {
                CommandError::PullRequestMerged => {
                    HttpError::Forbidden(CommandError::PullRequestMerged.to_string())
                }
                CommandError::PullRequestClosed(e_interno) => {
                    HttpError::Forbidden(CommandError::PullRequestClosed(e_interno).to_string())
                }
                CommandError::InvalidBranchName(branch) => {
                    HttpError::Forbidden(format!("{} no es una rama existente", branch))
                }
                CommandError::NothingToCompare(e_interno) => {
                    HttpError::Forbidden(CommandError::NothingToCompare(e_interno).to_string())
                }
                e => HttpError::InternalServerError(e),
            })?
            .ok_or(HttpError::NotFound)?;
        let response_body = serde_json::to_string(&saved_pull_request).unwrap();
        self.send_response(&200, "OK", &HashMap::new(), &response_body)
            .map_err(|e| HttpError::InternalServerError(e))?;

        Ok(())
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

    fn handle_get_pull_requests(
        &mut self,
        repo_path: &str,
        variables: &str,
    ) -> Result<(), HttpError> {
        let mut variables_map = HashMap::<String, String>::new();
        for variable in variables.split('&') {
            if variable.is_empty() {
                continue;
            }
            let (key, value) = variable.split_once('=').unwrap();
            _ = variables_map.insert(key.trim().to_string(), value.trim().to_string());
        }
        let state = variables_map
            .get("state")
            .get_or_insert(&"open".to_string())
            .to_owned();
        let mut sink = std::io::sink();
        let mut repo: GitRepository<'_> = self.get_repo(repo_path, &mut sink)?;
        let pull_requests = repo
            .get_pull_requests(&state)
            .map_err(|e| HttpError::InternalServerError(e))?;
        let response_body = serde_json::to_string(&pull_requests).unwrap();
        self.send_response(&200, "OK", &HashMap::new(), &response_body)
            .map_err(|e| HttpError::InternalServerError(e))
    }

    fn handle_get_pull_request(
        &mut self,
        repo_path: &str,
        pull_request_id: u64,
    ) -> Result<(), HttpError> {
        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let pull_request = repo
            .get_pull_request(pull_request_id)
            .map_err(|e| HttpError::InternalServerError(e))?;
        match pull_request {
            None => Err(HttpError::NotFound),
            Some(pull_request) => {
                let response_body = serde_json::to_string(&pull_request).unwrap();
                self.send_response(&200, "OK", &HashMap::new(), &response_body)
                    .map_err(|e| HttpError::InternalServerError(e))
            }
        }
    }

    fn handle_put_pull_request(
        &mut self,
        repo_path: &str,
        pull_request_id: u64,
    ) -> Result<(), HttpError> {
        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let pull_request = repo
            .get_pull_request(pull_request_id)
            .map_err(|e| HttpError::InternalServerError(e))?;
        match pull_request {
            None => Err(HttpError::NotFound),
            Some(mut pull_request) => {
                if pull_request.is_merged()? {
                    return Err(HttpError::Forbidden(
                        "Pull request is already merged".to_string(),
                    ));
                }
                if let PullRequestState::Closed = pull_request.get_state() {
                    return Err(HttpError::Forbidden("Pull request is closed".to_string()));
                }
                let message = format!(
                    "Merge pull request #{} from {}\n\n{}\n{}",
                    pull_request.id.ok_or(HttpError::InternalServerError(
                        CommandError::PullRequestUnknownID
                    ))?,
                    pull_request.source_branch,
                    pull_request.title,
                    pull_request.description
                );
                repo.try_merge_without_conflicts(
                    &pull_request.source_branch,
                    &pull_request.target_branch,
                    message,
                )
                .map_err(|e| HttpError::InternalServerError(e))?;
                pull_request.set_state(PullRequestState::Closed);
                pull_request.set_merged(true);
                pull_request.has_merge_conflicts = None;
                repo.save_pull_request(&mut pull_request)
                    .map_err(|e| HttpError::InternalServerError(e))?;

                let response_body = serde_json::to_string(&pull_request).unwrap();
                self.send_response(&200, "OK", &HashMap::new(), &response_body)
                    .map_err(|e: CommandError| HttpError::InternalServerError(e))
            }
        }
    }

    fn handle_get_pull_request_commits(
        &mut self,
        repo_path: &str,
        pull_request_id: u64,
    ) -> Result<(), HttpError> {
        let mut sink = std::io::sink();
        let mut repo = self.get_repo(repo_path, &mut sink)?;
        let commits = repo
            .get_pull_request_commits(pull_request_id)
            .map_err(|e| HttpError::InternalServerError(e))?;
        match commits {
            None => Err(HttpError::NotFound),
            Some(commits) => {
                let commits = commits
                    .into_iter()
                    .map(|commit| SimplifiedCommitObject::from_commit(commit))
                    .collect::<Vec<SimplifiedCommitObject>>();
                let response_body = serde_json::to_string(&commits).unwrap();
                self.send_response(&200, "OK", &HashMap::new(), &response_body)
                    .map_err(|e| HttpError::InternalServerError(e))
            }
        }
    }
}

// struct VerboseReader<'a> {
//     reader: &'a mut dyn std::io::Read,
// }

// impl<'a> VerboseReader<'a> {
//     pub fn new(reader: &'a mut dyn std::io::Read) -> Self {
//         Self { reader }
//     }
// }

// impl<'a> std::io::Read for VerboseReader<'a> {
//     fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//         println!("Reading ...");
//         let read = self.reader.read(buf)?;
//         let string = String::from_utf8_lossy(&buf[..read]);
//         println!("⬆️: {}", string);
//         Ok(read)
//     }
// }
