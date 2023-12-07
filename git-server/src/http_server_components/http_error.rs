use git_lib::command_errors::CommandError;

pub enum HttpError {
    InternalServerError(CommandError),
    BadRequest,
    NotFound,
}

impl HttpError {
    pub fn code(&self) -> u16 {
        match self {
            HttpError::InternalServerError(_) => 500,
            HttpError::BadRequest => 400,
            HttpError::NotFound => 404,
        }
    }

    pub fn message(&self) -> String {
        match self {
            HttpError::InternalServerError(error) => format!("Internal Server Error: {}", error),
            HttpError::BadRequest => "Bad Request".to_string(),
            HttpError::NotFound => "Not Found".to_string(),
        }
    }

    pub fn body(&self) -> String {
        match self {
            HttpError::InternalServerError(error) => error.to_string(),
            HttpError::BadRequest => "Invalid HTTP method".to_string(),
            HttpError::NotFound => "Not Found".to_string(),
        }
    }
}
