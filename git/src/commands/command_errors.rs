use std::{error::Error, fmt};

/// Enumeración de errores de flags
#[derive(Debug)]
pub enum CommandError {
    /// El nombre del comando no es válido
    Name,
    /// La flag no es válida
    WrongFlag,
    /// Argumentos inválidos
    InvalidArguments,
    /// Tipo de objeto no válido
    ObjectTypeError,
    /// No se encuentra el archivo
    FileNotFound(String),
    /// Hay un error leyendo el archivo
    FileReadError(String),
    /// Hay un error escribiendo el archivo
    FileWriteError(String),
    /// Hay un error abriendo el archivo
    FileOpenError(String),
    /// Error de compresión
    CompressionError,
    /// No se encuentra el directorio
    DirNotFound(String),
    /// No se pudo crear el directorio
    DirectoryCreationError(String),
    /// No se pudo crear el archivo
    FileCreationError(String),
}

impl Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::Name => write!(f, "El nombre del comando no es válido"),
            CommandError::WrongFlag => write!(f, "La flag no es válida"),
            CommandError::InvalidArguments => write!(f, "Argumentos inválidos"),
            CommandError::ObjectTypeError => write!(f, "Tipo de objeto no válido"),
            CommandError::FileNotFound(path) => write!(f, "No se encuentra el archivo: {path}"),
            CommandError::FileReadError(path) => {
                write!(f, "Hay un error leyendo el archivo: {path}")
            }
            CommandError::FileWriteError(path) => {
                write!(f, "Hay un error escribiendo el archivo: {path}")
            }
            CommandError::FileOpenError(path) => {
                write!(f, "Hay un error abriendo el archivo: {path}")
            }
            CommandError::CompressionError => write!(f, "Error de compresión"),
            CommandError::DirNotFound(path) => write!(f, "No se encuentra el directorio: {path}"),
            CommandError::DirectoryCreationError(path) => write!(f, "No se pudo crear el directorio: {path}"),
            CommandError::FileCreationError(path) => write!(f, "No se pudo crear el archivo: {path}"),

        }
    }
}
