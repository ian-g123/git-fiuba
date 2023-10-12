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
    /// Hay un error abriendo el archivo
    FileOpenError(String),
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
            CommandError::FileOpenError(path) => {
                write!(f, "Hay un error abriendo el archivo: {path}")
            }
        }
    }
}
