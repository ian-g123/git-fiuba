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
    MessageNoValue,
    InvalidAuthor,
    ReuseMessageNoValue,
    CommitLookUp(String),
    /// Error al abrir el staging area
    FailToOpenStaginArea(String),
    /// Error al guardar el staging area
    FailToSaveStaginArea(String),
    /// Es un archivo untracked.
    UntrackedError(String),

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
    ObjectPathError,
    FileNameError,
    /// No existe configuración de ususario.
    UserConfigurationError,
    FailToRecreateStagingArea,
    /// Se intentó agregar un archivo dentro de un archivo
    ObjectNotTree,
    StdinError,
    InvalidArgument(String),
    /// No se pudo conectar al servidor
    Connection(String),
    /// Error al leer un pkt
    ErrorReadingPkt,
    /// Error al leer un pkt con msg
    ErrorReadingPktVerbose(String),
    /// Error al enviar un mensaje
    SendingMessage(String),
    /// Error al intentar abrir el archivo de configuración
    InvalidConfigFile,
    /// No se encontró la url del repositorio remoto
    NoRemoteUrl,
    /// Nombre de rama inválido
    InvalidRefName,
    /// Tipo de objeto desconocido en packfile
    UnknownObjectType,
    /// Error al extraer datos de un un packfile
    ErrorExtractingPackfile,
    CastingError,
    MessageIncomplete(String),
    AllAndFilesFlagsCombination(String),
    /// No se pudo obtener el commit de HEAD
    NoHeadCommit(String),
    /// Error al intentar unir paths
    JoiningPaths,
    FailedToFindCommonAncestor,
    /// Ocurre un error al encontrar las ramas de los commits en push
    PushBranchesError,
    /// Error al obtener el tree desde el option que debería ser tree en push
    PushTreeError,
    PushBranchBehindVerbose(String, String),
    /// Octopus merge not supported
    MergeMultipleCommits,
    /// Merge conflict
    MergeConflict(String),
    /// You can only use merge with one option --continue | --abort | --quit
    MergeOneOperation,
    /// There is no merge to continue, abort or quit
    NoMergeFound,
    /// Couldn't continue with automerge
    FailedToResumeMerge,
    /// error: Committing is not possible because you have unmerged files.
    UnmergedFiles,
    /// There cannot be a file and a folder with the same name
    CannotHaveFileAndFolderWithSameName(String),

    PushBranchBehind(String),

    // InterfaceError(String),
    /// Error de I/O
    Io {
        message: String,
        error: String,
    },
    /// Error al negociar paquetes con cliente
    PackageNegotiationError(String),
    /// Error al intentar leer un archivo
    CheckingCommitsBetweenError(String),
    /// Error al eliminar un archivo
    FileRemovingError(String),
    /// Error de recursividad de comando
    NotRecursive(String),
    RmFromStagingAreaError(String),
    PullError(String),

    // Branch errors
    /// fatal: The -a, and -r, options to 'git branch' do not take a branch name.
    CreateAndListError,
    /// fatal: cannot use -a with -d
    ShowAllAndDelete,
    /// fatal: branch name required
    BranchNameRequired,
    /// Se intentó usar -m y -D
    RenameAndDelete,
    /// No se puede renombrar una rama que no existe
    NoOldBranch(String),
    /// No se puede renombrar una rama con un nombre que existe
    NewBranchExists(String),
    /// branch -m solo recibe 2 nombres
    FatalRenameOperation,
    /// No se pudo crear la branch
    FatalCreateBranchOperation,
    /// Nombre de objeto inválido. No se puede crear la rama
    InvalidObjectName(String),
    /// No se puede crear una rama que ya existe
    BranchExists(String),
    /// Nombre de rama inválido.
    InvalidBranchName(String),
    /// Ocurrió un error al eliminar el directorio
    RemoveDirectoryError(String),
    /// Ocurrió un error al eliminar el archivo
    RemoveFileError(String),
    /// Se usó el flag -D de branch sin argumentos
    DeleteWithNoArgs,

    // Checkout
    /// fatal: Cannot update paths and switch to branch 'b3' at the same time.
    UpdateAndSwicth(String),
    /// error: switch `b' requires a value
    SwitchRequiresValue,
    CheckoutConflictsError,
    /// El tree guarda solo hashes y no sus objetos
    ShallowTree,

    MergeConflictsCommit,
    /// Errror in object decompression
    ErrorDecompressingObject(String),
    /// No se pudo obtener el sender del logger
    NotValidLogger,

    // Tags
    TagNameDuplicated(String),
    TagNameNeeded,
    TagTooManyArgs,
    TagCreateAndDelete,
    TagMessageEmpty,
    TagAlreadyExists(String),
    InvalidRef(String),
    RebaseContinueError,
    RebaseMergeConflictsError,
    RebaseError(String),

    // Show-ref
    FlagHashRequiresValue,

    // Log
    ReadRefsHeadError,

    // Not a valid tag
    TagNotFound(String),
    // La tag que usaste no apunta a un commit
    MergeTagNotCommit(String),
    // No se implementó está funcionalidad
    FeatureNotImplemented(String),

    // Ls-tree
    LsTreeErrorNotATree,
    // Error al añadir un archivo
    AddStagingAreaError(String, String),

    //index
    MetadataError(String),

    // check-ignore
    StdinAndPathsError,
    NoPathSpecified,
    NonMatchingWithoutVerbose,
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
            CommandError::MessageNoValue => write!(f, "error: switch `m' requires a value"),
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
                write!(f, "No se pudo obtener el nombre del archivo.")
            }
            CommandError::ObjectPathError => {
                write!(f, "No se pudo obtener el path del objeto.")
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
            CommandError::InvalidArgument(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::Connection(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::ErrorReadingPkt => {
                write!(f, "Error al leer un pkt")
            }

            CommandError::ErrorReadingPktVerbose(msg) => {
                write!(f, "Error al leer un pkt: {}", msg)
            }
            CommandError::SendingMessage(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::InvalidConfigFile => {
                write!(f, "Error al intentar abrir el archivo de configuración")
            }
            CommandError::NoRemoteUrl => {
                write!(f, "No se encontró la url del repositorio remoto")
            }
            CommandError::InvalidRefName => {
                write!(f, "Nombre de rama inválido")
            }
            CommandError::UnknownObjectType => {
                write!(f, "Tipo de objeto desconocido")
            }
            CommandError::ErrorExtractingPackfile => {
                write!(f, "Error al extraer datos de un un packfile")
            }

            CommandError::CastingError => {
                write!(f, "Casting error")
            }
            CommandError::UntrackedError(path) => {
                write!(
                    f,
                    "error: pathspec '{}' did not match any file(s) known to git",
                    path
                )
            }
            CommandError::MessageIncomplete(end) => {
                write!(f, "The message must end with {}", end)
            }

            CommandError::AllAndFilesFlagsCombination(path) => {
                write!(f, "fatal: paths '{} ...' with -a does not make sense", path)
            }
            CommandError::NoHeadCommit(name) => {
                write!(f, "No se pudo obtener el commit de HEAD: \"{}\"", name)
            }
            CommandError::JoiningPaths => {
                write!(f, "Error al intentar unir paths")
            }
            CommandError::FailedToFindCommonAncestor => {
                write!(f, "No se pudo encontrar un ancestro común.")
            }
            CommandError::PushBranchesError => {
                write!(
                    f,
                    "Ocurre un error al encontrar las ramas de los commits en push"
                )
            }
            CommandError::PushTreeError => {
                write!(
                    f,
                    "Error al obtener el tree desde el option que debería ser tree en push"
                )
            }
            CommandError::PushBranchBehindVerbose(url, branch) => {
                write!(
                    f,
                    "! [rejected]        {branch} -> {branch} (non-fast-forward)\nerror: failed to push some refs to '{url}'\n"
                )
            }
            CommandError::MergeMultipleCommits => {
                write!(f, "Octopus merge not supported")
            }
            CommandError::MergeConflict(explanation) => {
                write!(f, "Merge conflict! Error: {}", explanation)
            }
            CommandError::MergeOneOperation => {
                write!(
                    f,
                    "You can only use merge with one option --continue | --abort | --quit"
                )
            }
            CommandError::NoMergeFound => {
                write!(f, "There is no merge to continue, abort or quit")
            }
            CommandError::FailedToResumeMerge => {
                write!(f, "Couldn't continue with automerge")
            }
            CommandError::UnmergedFiles => {
                write!(f, "error: Committing is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: Exiting because of an unresolved conflict.")
            }
            CommandError::CannotHaveFileAndFolderWithSameName(path) => {
                write!(
                    f,
                    "There cannot be a file and a folder with the same name: {}",
                    path
                )
            }
            CommandError::PushBranchBehind(local_branch) => {
                write!(f, "error: failed to push some refs to {}", local_branch)
            }
            // CommandError::InterfaceError(msg) => {
            //     write!(f, "Ocurrió un error en la interfaz: {}", msg)
            // }
            CommandError::Io { message, error } => {
                write!(f, "{}: {}", message, error)
            }
            CommandError::PackageNegotiationError(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::NotRecursive(path) => {
                write!(f, "No se remueve {path} recursivamente sin el flag -r")
            }
            CommandError::FileRemovingError(path) => {
                write!(f, "Hay un error cerrando el archivo: {path}")
            }
            CommandError::RmFromStagingAreaError(path) => {
                write!(
                    f,
                    "No se puede remover un archivo que no fue agregado al Staging Area: {path}"
                )
            }
            CommandError::PullError(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::CreateAndListError => {
                write!(
                    f,
                    "fatal: The -a, and -r, options to 'git branch' do not take a branch name."
                )
            }
            CommandError::ShowAllAndDelete => write!(f, "fatal: cannot use (-a | -all) with -D"),
            CommandError::RenameAndDelete => write!(f, "fatal: cannot use -m with -D"),

            CommandError::BranchNameRequired => write!(f, "fatal: branch name required"),
            CommandError::NoOldBranch(name) => {
                write!(
                    f,
                    "error: refname refs/heads/{}\nfatal: Branch rename failed",
                    name
                )
            }
            CommandError::NewBranchExists(name) => {
                write!(f, "fatal: A branch named '{name}' already exists")
            }
            CommandError::FatalRenameOperation => {
                write!(f, "fatal: too many arguments for a rename operation")
            }
            CommandError::InvalidObjectName(name) => {
                write!(f, "fatal: Not a valid object name: '{name}'.")
            }
            CommandError::FatalCreateBranchOperation => {
                write!(f, "fatal: too many arguments for a create operation")
            }
            CommandError::BranchExists(name) => {
                write!(f, "fatal: A branch named '{name}' already exists.")
            }
            CommandError::InvalidBranchName(name) => {
                write!(f, "fatal: '{name}' is not a valid branch name.")
            }
            CommandError::RemoveDirectoryError(error) => {
                write!(f, "Error: {error}")
            }
            CommandError::RemoveFileError(error) => {
                write!(f, "Error: {error}")
            }
            CommandError::DeleteWithNoArgs => write!(f, "fatal: branch name required"),
            CommandError::UpdateAndSwicth(branch) => write!(
                f,
                "fatal: Cannot update paths and switch to branch '{branch}' at the same time."
            ),
            CommandError::SwitchRequiresValue => write!(f, "error: switch `b' requires a value"),
            CommandError::CheckoutConflictsError => {
                write!(f, "No se puede cambiar de rama. Hay conflictos")
            }
            CommandError::CheckingCommitsBetweenError(msg) => {
                write!(f, "{}", msg)
            }
            CommandError::ShallowTree => {
                write!(f, "El tree guarda solo hashes y no sus objetos")
            }
            CommandError::MergeConflictsCommit => write!(f, "error: Committing is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: Exiting because of an unresolved conflict."),
            CommandError::ErrorDecompressingObject(msg) => {
                write!(f, "Errror in object decompression: {}", msg)
            }
            CommandError::NotValidLogger => {
                write!(f, "No se pudo obtener el sender del logger")
            }
            CommandError::TagNameDuplicated(name) => write!(f, "fatal: tag '{name}' already exists"),
            CommandError::TagNameNeeded => write!(f, "No se puede crear un tag sin un nombre"),
            CommandError::TagTooManyArgs => write!(f, "fatal: too many arguments"),
            CommandError::TagCreateAndDelete => write!(f, "No se puede crear y eliminar tags al mismo tiempo"),
            CommandError::TagMessageEmpty => write!(f, "fatal: no tag message?"),
            CommandError::TagAlreadyExists(tag) => write!(f, "fatal: tag '{tag}' already exists"),
            CommandError::InvalidRef(tag_ref) => write!(f, "fatal: Failed to resolve '{tag_ref}' as a valid ref."),
            CommandError::RebaseContinueError => write!(f, "No se puede hacer rebase, hay conflictos de merge"), //PONER BIEN MSJ!!!!!!!!!!!!!!!!!!!!!!!!
            CommandError::RebaseMergeConflictsError => write!(f, "Resolver Conflictos"), //PONER BIEN MSJ!!!!!!!!!!!!!!!!!!!!!!!!
            CommandError::RebaseError(msj) => write!(f, "{msj}"),
            CommandError::FlagHashRequiresValue => write!(f, "error: option `hash' expects a numerical value"),
            CommandError::ReadRefsHeadError => write!(f, "Error al leer el archivo .git/refs/heads/HEAD"),
            CommandError::TagNotFound(tag_name) => write!(f, "fatal: tag not found: {tag_name}"),
            CommandError::MergeTagNotCommit(tag_name) => write!(f, "La tag {} no apunta a un commit", tag_name),
            CommandError::FeatureNotImplemented(feature) => write!(f, "Feature not implemented: {}", feature),
            CommandError::LsTreeErrorNotATree => write!(f, "fatal: not a tree object"),
            CommandError::AddStagingAreaError(path,e) => write!(f, "Error al añadir el archivo {}: {}",path, e),
            CommandError::MetadataError(e) => write!(f, "Error de metadatos: {}", e),
            CommandError::StdinAndPathsError => write!(f, "fatal: cannot specify pathnames with --stdin"),
            CommandError::NoPathSpecified => write!(f, "fatal: no path specified"),
            CommandError::NonMatchingWithoutVerbose => write!(f, "fatal: --non-matching is only valid with --verbose"),
        }
    }
}
