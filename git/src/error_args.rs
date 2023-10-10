use std::fmt::{self};
use std::error;

#[derive(Debug, Clone)]
pub enum ErrorFlags {
    ArgsNumber,
    CommandName,
    InvalidFlag,
}

impl fmt::Display for ErrorFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ErrorFlags")
    }
}

impl error::Error for ErrorFlags {}
