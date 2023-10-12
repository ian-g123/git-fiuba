#[derive(Debug)]
pub enum ErrorFlags {
    CommandName,
    WrongFlag,
    InvalidArguments,
    FileNotFound,
    FileReadError,
    ObjectTypeError,
}
