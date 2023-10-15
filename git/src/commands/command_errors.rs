use std::{error::Error, fmt::{self, write}, path::PathBuf};

/// Enumeración de errores de flags
#[derive(Debug, PartialEq)]
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
    /// No es un repositorio de Git.
    NotGitRepository,

    // Commit Errors

    /// El flag -m de Commit no se puede combinar con -C.
    MessageAndReuseError,
    CommitMessageEmptyValue,
    CommitMessageNoValue,
    ReuseMessageNoValue,
    CommitLookUp(String),
    /// Error al abrir el staging area
    FailToOpenSatginArea(String),
    /// Error al guardar el staging area
    FailToSaveStaginArea(String),

    CurrentDirectoryError,
    HeadError,
    InvalidDirectory,
    InvalidDirectoryEntry,
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
            CommandError::MessageAndReuseError => write!(f, "fatal: Option -m cannot be combined with -C"),
            CommandError::CommitMessageNoValue => write!(f,"error: switch `m' requires a value"),
            CommandError::CommitMessageEmptyValue => write!(f, "Aborting commit due to empty commit message."),
            CommandError::ReuseMessageNoValue => write!(f, "error: switch `C' requires a value"),
            CommandError::CommitLookUp(hash) => write!(f, "fatal: could not lookup commit {hash}"),
            CommandError::NotGitRepository => write!(f, "fatal: not a git repository (or any of the parent directories): .git"),
            CommandError::FailToOpenSatginArea(error) => {
                write!(f, "Error al abrir el staging area: {error}")
            }
            CommandError::FailToSaveStaginArea(error) => {
                write!(f, "Error al guardar el staging area: {error}")
            },
            CommandError::CurrentDirectoryError => {
                write!(f, "Current directory does not existo or there are insufficient permissions to access the current directory")
            },
            CommandError::HeadError => {
                write!(f, "El archivo .git/HEAD tiene formato inválido")
            },
            CommandError::InvalidDirectoryEntry => {
                write!(f, "Entrada de directorio inválida")
            },
            CommandError::InvalidDirectory => {
                write!(f, "Directorio inválido")
            },
        }
    }
}
