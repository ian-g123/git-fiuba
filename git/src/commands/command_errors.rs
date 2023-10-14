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
    /// Nombre de archivo inválido
    InvalidFileName,
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
    /// No se proporcionaron suficientes argumentos para este comando
    NotEnoughArguments,
    /// El flag -e no se utiliza comúnmente junto con otros flags en el comando
    OptionCombinationError,
    /// Error al abrir el staging area
    FailToOpenSatginArea(String),
    /// Error al guardar el staging area
    FailToSaveStaginArea(String),
}

impl Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::Name => write!(f, "El nombre del comando no es válido"),
            CommandError::WrongFlag => write!(f, "La flag no es válida"),
            CommandError::InvalidArguments => write!(f, "Argumentos inválidos"),
            CommandError::ObjectTypeError => write!(f, "Tipo de objeto no válido"),
            CommandError::InvalidFileName => write!(f, "Nombre de archivo inválido"),
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
            CommandError::NotEnoughArguments => write!(
                f,
                "No se proporcionaron suficientes argumentos para este comando"
            ),
            CommandError::OptionCombinationError => write!(
                f,
                "El flag -e no se utiliza comúnmente junto con otros flags en el comando"
            ),
            CommandError::FailToOpenSatginArea(error) => {
                write!(f, "Error al abrir el staging area: {error}")
            }
            CommandError::FailToSaveStaginArea(error) => {
                write!(f, "Error al guardar el staging area: {error}")
            }
        }
    }
}
