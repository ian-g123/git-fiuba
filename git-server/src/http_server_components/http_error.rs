use git_lib::command_errors::CommandError;

pub enum HttpError {
    InternalServerError(CommandError),
    BadRequest(String),
    NotFound,
}

impl HttpError {
    pub fn code(&self) -> u16 {
        match self {
            HttpError::InternalServerError(_) => 500,
            HttpError::BadRequest(_) => 400,
            HttpError::NotFound => 404,
        }
    }

    pub fn message(&self) -> String {
        match self {
            HttpError::InternalServerError(error) => format!("Internal Server Error: {}", error),
            HttpError::BadRequest(e) => format!("Bad Request: {}", e),
            HttpError::NotFound => "Not Found".to_string(),
        }
    }

    pub fn body(&self) -> String {
        match self {
            HttpError::InternalServerError(error) => error.to_string(),
            HttpError::BadRequest(_) => "Invalid HTTP method".to_string(),
            HttpError::NotFound => "Not Found".to_string(),
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let code = self.code();
        let message = self.message();
        let body = self.body();
        write!(f, "HTTP Error {}: {}\n{}", code, message, body)
    }
}
