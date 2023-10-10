#[derive(Debug)]
pub enum ErrorFlags {
    CommandName,
    InvalidArguments,
    FileNotFound,
    FileReadError,
}
