use std::{
    error::Error,
    fmt::{self},
};

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
    /// No se encuentra el directorio
    DirNotFound(String),
    /// No se pudo crear el directorio
    DirectoryCreationError(String),
    /// No se pudo crear el archivo
    FileCreationError(String),
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
    InvalidAuthor,
    ReuseMessageNoValue,
    CommitLookUp(String),
    /// Error al abrir el staging area
    FailToOpenStaginArea(String),
    /// Error al guardar el staging area
    FailToSaveStaginArea(String),

    CurrentDirectoryError,
    HeadError,
    InvalidDirectory,
    InvalidDirectoryEntry,
    InvalidCommit,
    /// Error al intentar agregar a un arbol un blob cuyo path no es subdirectorio del arbol
    NotYourFather,
    /// Error al intentar parsear la longitud en el header de un objeto
    ObjectLengthParsingError,
    /// Error al intentar calcular el tamaño de un objeto
    FailToCalculateObjectSize,
    /// Error al intentar buscar el hash de un objeto
    ObjectHashNotKnown,
    /// Modo de archivo inválido.
    InvalidMode,
    /// No se pudo obtener el nombre del objeto.
    FileNameError,
    /// No existe configuración de ususario.
    UserConfigurationError,
    FailToRecreateStagingArea,
    /// Se intentó agregar un archivo dentro de un archivo
    ObjectNotTree,
    StdinError,
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
            CommandError::DirNotFound(path) => write!(f, "No se encuentra el directorio: {path}"),
            CommandError::DirectoryCreationError(path) => {
                write!(f, "No se pudo crear el directorio: {path}")
            }
            CommandError::FileCreationError(path) => {
                write!(f, "No se pudo crear el archivo: {path}")
            }

            CommandError::NotEnoughArguments => write!(
                f,
                "No se proporcionaron suficientes argumentos para este comando"
            ),
            CommandError::OptionCombinationError => write!(
                f,
                "El flag -e no se utiliza comúnmente junto con otros flags en el comando"
            ),
            CommandError::MessageAndReuseError => {
                write!(f, "fatal: Option -m cannot be combined with -C")
            }
            CommandError::CommitMessageNoValue => write!(f, "error: switch `m' requires a value"),
            CommandError::CommitMessageEmptyValue => {
                write!(f, "Aborting commit due to empty commit message.")
            }
            CommandError::ReuseMessageNoValue => write!(f, "error: switch `C' requires a value"),
            CommandError::CommitLookUp(hash) => write!(f, "fatal: could not lookup commit {hash}"),
            CommandError::NotGitRepository => write!(
                f,
                "fatal: not a git repository (or any of the parent directories): .git"
            ),
            CommandError::FailToOpenStaginArea(error) => {
                write!(f, "Error al abrir el staging area: {error}")
            }
            CommandError::FailToSaveStaginArea(error) => {
                write!(f, "Error al guardar el staging area: {error}")
            }
            CommandError::CurrentDirectoryError => {
                write!(f, "Current directory does not existo or there are insufficient permissions to access the current directory")
            }
            CommandError::HeadError => {
                write!(f, "El archivo .git/HEAD tiene formato inválido")
            }
            CommandError::InvalidDirectoryEntry => {
                write!(f, "Entrada de directorio inválida")
            }
            CommandError::InvalidDirectory => {
                write!(f, "Directorio inválido")
            }
            CommandError::InvalidCommit => {
                write!(f, "Commit inválido")
            }
            CommandError::InvalidAuthor => {
                write!(f, "Autor inválido")
            }
            CommandError::NotYourFather => {
                write!(f, "Error al intentar agregar a un arbol un blob cuyo path no es subdirectorio del arbol")
            }
            CommandError::ObjectLengthParsingError => {
                write!(
                    f,
                    "Error al intentar parsear la longitud en el header de un objeto"
                )
            }
            CommandError::FailToCalculateObjectSize => {
                write!(f, "Error al intentar calcular el tamaño de un objeto")
            }
            CommandError::ObjectHashNotKnown => {
                write!(f, "Error al intentar buscar el hash de un objeto")
            }
            CommandError::InvalidMode => {
                write!(f, "Modo de archivo inválido.")
            }
            CommandError::FileNameError => {
                write!(f, "No se pudo obtener el nombre del objeto.")
            }
            CommandError::UserConfigurationError => {
                write!(f, "No existe configuración de ususario.")
            }
            CommandError::FailToRecreateStagingArea => {
                write!(f, "Error al intentar recrear el staging area")
            }
            CommandError::ObjectNotTree => {
                write!(f, "Se intentó agregar un archivo dentro de un archivo")
            }
            CommandError::StdinError => {
                write!(f, "No se pudo leer por entrada estándar")
            }
        }
    }
}
