use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

use crate::commands::{
    add_components::add::{self},
    command::{Command, ConfigAdderFunction},
};
use git_lib::{
    branch_manager::get_last_commit,
    command_errors::CommandError,
    config::Config,
    git_repository::GitRepository,
    logger::Logger,
    objects::{
        author::Author,
        aux::get_name,
        blob::Blob,
        commit_object::{write_commit_tree_to_database, CommitObject},
        git_object::{GitObject, GitObjectTrait},
        last_commit::is_in_last_commit,
        tree::Tree,
    },
    objects_database,
    staging_area::StagingArea,
};

/// Hace referencia a un Comando Commit.
pub struct Commit {
    all: bool,
    reuse_message: Option<String>,
    dry_run: bool,
    message: Option<String>,
    quiet: bool,
    files: Vec<String>,
}

impl Command for Commit {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "commit" {
            return Err(CommandError::Name);
        }
        logger.log(&format!("commit args {:?}", args));

        let instance = Commit::new_from(args)?;

        instance.run(stdin, output, logger)?;

        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Commit> {
        vec![
            Commit::add_all_config,
            Commit::add_dry_run_config,
            Commit::add_message_config,
            Commit::add_quiet_config,
            Commit::add_reuse_message_config,
            Commit::add_pathspec_config,
        ]
    }
}

impl Commit {
    /// Crea un nuevo Comando Commit a partir de sus argumentos. Lo configura.
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut commit = Self::new_default();
        commit.config(args)?;
        Ok(commit)
    }

    /// Crea un nuevo Comando Commit a partir de valores por default.
    fn new_default() -> Self {
        Commit {
            all: false,
            reuse_message: None,
            dry_run: false,
            message: None,
            quiet: false,
            files: Vec::new(),
        }
    }

    /// Configura el flag -C.
    fn add_reuse_message_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options = ["-C".to_string(), "--reuse-message".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::ReuseMessageNoValue)?;
        self.reuse_message = Some(args[i + 1].clone());
        Ok(i + 2)
    }

    /// Configura el flag -m.
    fn add_message_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-m".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::CommitMessageNoValue)?;
        let mut new_message: String = String::new();
        if let Some(message) = &self.message {
            new_message = format!("{}\n\n", message)
        }
        new_message += &args[i + 1];
        self.message = Some(new_message);
        Ok(i + 2)
    }

    /// Configura el flag --dry-run.
    fn add_dry_run_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--dry-run".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.dry_run = true;
        Ok(i + 1)
    }

    /// Configura el flag -q.
    fn add_quiet_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-q".to_string(), "--quiet".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.quiet = true;
        Ok(i + 1)
    }

    /// Configura el flag (--all | -a).
    fn add_all_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-a".to_string(), "--all".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.all = true;
        Ok(i + 1)
    }

    /// Configura un Commit que recibe paths para commitear.
    fn add_pathspec_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::InvalidArguments);
        }
        self.files.push(args[i].clone());
        Ok(i + 1)
    }

    /// Devuelve true si el siguiente argumento es un flag.
    fn check_next_arg(
        &mut self,
        i: usize,
        args: &[String],
        error: CommandError,
    ) -> Result<(), CommandError> {
        if i >= args.len() - 1 || Self::is_flag(&args[i + 1]) {
            return Err(error);
        }
        Ok(())
    }

    /// Comprueba si el flag es invalido. En ese caso, devuelve error.
    fn check_errors_flags(
        i: usize,
        args: &[String],
        options: &[String],
    ) -> Result<(), CommandError> {
        if !options.contains(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        Ok(())
    }

    /// Lee el mensaje introducido por el usuario por entrada estandar.
    fn run_enter_message(stdin: &mut dyn Read) -> Result<String, CommandError> {
        let stdout = get_enter_message_text()?;
        println!("{}#\n", stdout);
        let mut message = read_from_stdin(stdin)?;
        message = ignore_commented_lines(message);

        if message.is_empty() {
            return Err(CommandError::CommitMessageEmptyValue);
        }

        Ok(message.trim().to_string())
    }

    /// Devuelve el mesage del Commit. Si se usó el flag -m, devuelve el mensaje asociado.\
    /// Si hay que reusar el de otro commit (-C), devuelve un string vacío.\
    /// Si no se ha usado ninguno de esos flags, se pide al usuario que introduzca el mensaje nuevamente.
    fn get_commit_message(&self, stdin: &mut dyn Read) -> Result<String, CommandError> {
        let message = {
            if let Some(message) = self.message.clone() {
                message
            } else if self.reuse_message.is_some() {
                "".to_string()
            } else {
                Self::run_enter_message(stdin)?
            }
        };
        Ok(message)
    }

    /// Ejecuta el Comando Commit.
    fn run(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if self.message.is_some() && self.reuse_message.is_some() {
            return Err(CommandError::MessageAndReuseError);
        }
        logger.log("Retreiving message");

        let message = self.get_commit_message(stdin)?;
        logger.log("Opening stagin_area");

        let mut staging_area = StagingArea::open()?;
        logger.log("Staging area opened");

        if !self.files.is_empty() && self.all {
            return Err(CommandError::AllAndFilesFlagsCombination(
                self.files[0].clone(),
            ));
        }

        if !self.files.is_empty() {
            self.run_files_config(logger, &mut staging_area)?
        }
        if self.all {
            self.run_all_config(&mut staging_area, logger)?;
        }

        if !staging_area.has_changes(logger)? {
            logger.log("Nothing to commit");
            // show status output + no changes added to commit (use "git add" and/or "git commit -a")
            return Ok(());
        } else {
            self.run_commit(logger, message, &mut staging_area)?;
        }

        Ok(())
    }

    /// Si se han introducido paths como argumentos del comando, se eliminan los cambios
    /// guardados en el Staging Area y se agregan los nuevos.\
    /// Estos archivos deben ser reconocidos por git.
    fn run_files_config(
        &self,
        logger: &mut Logger,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        if !self.files.is_empty() {
            logger.log("Running pathspec configuration");
            let files = staging_area.get_files();

            for path in self.files.iter() {
                if !is_untracked(path, logger, &files)? {
                    let inner = Vec::new(); // Me obligan a entregar un output
                    let mut cursor = Cursor::new(inner);
                    let mut repo = GitRepository::open("", &mut cursor)?;
                    repo.add_file(path, staging_area)?;
                    // add::run_for_file(path, staging_area, logger)?;
                } else {
                    return Err(CommandError::UntrackedError(path.to_owned()));
                }
            }
            staging_area.save()?;
        }
        Ok(())
    }

    /// Guarda en el staging area todos los archivos modificados y elimina los borrados.\
    /// Los archivos untracked no se guardan.
    fn run_all_config(
        &self,
        staging_area: &mut StagingArea,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        logger.log("Running 'all' configuration\n");
        let files = &staging_area.get_files();
        staging_area.empty(logger)?;
        for (path, _) in staging_area.get_files() {
            if !Path::new(&path).exists() {
                staging_area.remove(&path);
            }
        }
        save_entries("./", staging_area, logger, files)?;
        staging_area.save()?;
        Ok(())
    }

    /// Ejecuta la creación del Commit.
    fn run_commit(
        &self,
        logger: &mut Logger,
        message: String,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let last_commit_hash = get_last_commit()?;

        let mut parents: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            parents.push(padre);
        }

        logger.log("Creating Index tree");

        let mut staged_tree = {
            if self.files.is_empty() {
                staging_area.get_working_tree_staged(logger)?
            } else {
                staging_area.get_working_tree_staged_bis(logger, self.files.clone())?
            }
        };

        logger.log("Index tree created");

        let commit: CommitObject =
            self.get_commit(&message, parents, staged_tree.to_owned(), logger)?;

        let mut git_object: GitObject = Box::new(commit);
        write_commit_tree_to_database(&mut staged_tree, logger)?;

        if !self.dry_run {
            let commit_hash = objects_database::write(logger, &mut git_object)?;
            logger.log(&format!("Commit object saved in database {}", commit_hash));
            logger.log(&format!("Updating last commit to {}", commit_hash));

            update_last_commit(&commit_hash)?;
            logger.log("Last commit updated");
            // show commit status
        }

        // if !self.quiet {
        //     //self.get_commit_output(commit)
        // }

        Ok(())
    }

    /// Obtiene el objeto Commit y lo devuelve.
    fn get_commit(
        &self,
        message: &str,
        parents: Vec<String>,
        staged_tree: Tree,
        logger: &mut Logger,
    ) -> Result<CommitObject, CommandError> {
        let commit: CommitObject = {
            if let Some(commit_hash) = &self.reuse_message {
                Self::get_reused_commit(commit_hash.to_string(), parents, staged_tree, logger)?
            } else {
                Self::create_new_commit(message.to_owned(), parents, staged_tree, logger)?
            }
        };

        logger.log("Commit object created");
        Ok(commit)
    }

    /// Crea un nuevo objeto Commit a partir de la información pasada.
    fn create_new_commit(
        message: String,
        parents: Vec<String>,
        mut staged_tree: Tree,
        logger: &mut Logger,
    ) -> Result<CommitObject, CommandError> {
        let staged_tree_hash = staged_tree.get_hash_string()?;
        let config = Config::open()?;

        let Some(author_email) = config.get("user.email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(author_name) = config.get("user.name") else {
            return Err(CommandError::UserConfigurationError);
        };

        let author = Author::new(author_name, author_email);
        let commiter = Author::new(author_name, author_email);

        let datetime: DateTime<Local> = Local::now();
        let timestamp = datetime.timestamp();
        let offset = datetime.offset().local_minus_utc() / 60;
        logger.log(&format!("offset: {}", offset));
        let commit = CommitObject::new(
            parents,
            message,
            author,
            commiter,
            timestamp,
            offset,
            staged_tree,
            None,
        )?;
        Ok(commit)
    }

    /// Crea un objeto Commit a partir de los datos de otro Commit.
    fn get_reused_commit(
        commit_hash: String,
        parents: Vec<String>,
        staged_tree: Tree,
        logger: &mut Logger,
    ) -> Result<CommitObject, CommandError> {
        let mut other_commit = objects_database::read_object(&commit_hash, logger)?;
        let hash_commit = other_commit.get_hash()?;
        if let Some((message, author, committer, timestamp, offset)) =
            other_commit.get_info_commit()
        {
            let commit = CommitObject::new(
                parents,
                message,
                author,
                committer,
                timestamp,
                offset,
                staged_tree,
                Some(hash_commit),
            )?;
            return Ok(commit);
        }
        Err(CommandError::CommitLookUp(commit_hash))
    }

    /// Obtiene la salida por stdout del comando Commit.
    fn get_status_output(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        /*
        si el staging area está vacía, se usa el output de status.
         */

        /* let mut status = Status::new_default();
        status.get_output(output)?; */
        Ok(())
    }
}

/// Devuelve el texto que se mostrará si el Cliente no ha introducido un mensaje para el Commit.
fn get_enter_message_text() -> Result<String, CommandError> {
    let mensaje = "# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n#\n";
    /* let branch_path = get_current_branch()?;
    let branch_split: Vec<&str> = branch_path.split("/").collect();
    let branch_name = branch_split[branch_split.len() - 1]; */
    Ok(format!("{}#\n# Output de status\n#\n", mensaje))
}

/// Lee por stdin y guarda el mensaje introducido.
fn read_from_stdin(stdin: &mut dyn Read) -> Result<String, CommandError> {
    let mut message = String::new();
    let end = "q".to_string();
    loop {
        let mut buf = [0; 1];
        if stdin.read_exact(&mut buf).is_err() {
            return Err(CommandError::StdinError);
        };
        let input = String::from_utf8_lossy(&buf).to_string();
        if check_end_message(&input, &end) {
            break;
        }
        message += &input;
    }
    Ok(message)
}

/// Comprueba si el cliente ha terminado de introducir el mensaje.
fn check_end_message(message: &str, end: &str) -> bool {
    let split_message: Vec<String> = message.lines().map(String::from).collect();
    if let Some(last) = split_message.to_owned().last() {
        if last.to_owned() == end.to_string() {
            return true;
        }
    }
    false
}

/// Devuelve un String sin las líneas que empiezan con '#'.
fn ignore_commented_lines(message: String) -> String {
    let split_message: Vec<&str> = message
        .lines()
        .filter(|line| !line.trim_start().starts_with("#"))
        .collect();
    split_message.join("\n")
}

/// Devuelve true si Git no reconoce el path pasado.
fn is_untracked(
    path: &str,
    logger: &mut Logger,
    staging_area: &HashMap<String, String>,
) -> Result<bool, CommandError> {
    let mut blob = Blob::new_from_path(path.to_string())?;
    let hash = &blob.get_hash_string()?;
    let (is_in_last_commit, name) = is_in_last_commit(hash.to_owned(), logger)?;
    if staging_area.contains_key(path) || (is_in_last_commit && name == get_name(&path)?) {
        logger.log("Staging area has this path");
        return Ok(false);
    }
    Ok(true)
}

/// Guarda en el stagin area el estado actual del working tree, sin tener en cuenta los archivos
/// nuevos.
fn save_entries(
    path_name: &str,
    staging_area: &mut StagingArea,
    logger: &mut Logger,
    files: &HashMap<String, String>,
) -> Result<(), CommandError> {
    let path = Path::new(path_name);

    let Ok(entries) = fs::read_dir(path.clone()) else {
        return Err(CommandError::DirNotFound(path_name.to_owned()));
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return Err(CommandError::DirNotFound(path_name.to_owned()));
        };
        let entry_path = entry.path();
        let entry_name = get_path_str(entry_path.clone())?;
        if entry_name.contains(".git") {
            continue;
        }
        logger.log(&format!("Saving entry name: {}", entry_name));
        if entry_path.is_dir() {
            save_entries(&entry_name, staging_area, logger, files)?;
            return Ok(());
        } else {
            let blob = Blob::new_from_path(entry_name.to_string())?;
            let path = &entry_name[2..];
            if !is_untracked(path, logger, files)? {
                logger.log(&format!("{} is tracked", entry_name));
                let mut git_object: GitObject = Box::new(blob);
                let hex_str = objects_database::write(logger, &mut git_object)?;
                staging_area.add(path, &hex_str);
            }
        }
    }
    Ok(())
}

/// Devuelve el nombre de un archivo o directorio dado un PathBuf.
fn get_path_str(path: PathBuf) -> Result<String, CommandError> {
    let Some(path_name) = path.to_str() else {
        return Err(CommandError::DirNotFound("".to_string())); //cambiar
    };
    Ok(path_name.to_string())
}

/// Actualiza la referencia de la rama actual al nuevo commit.
fn update_last_commit(commit_hash: &str) -> Result<(), CommandError> {
    let currect_branch = get_head_ref()?;
    let branch_path = format!(".git/{}", currect_branch);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(branch_path)
        .map_err(|_| CommandError::FileOpenError(currect_branch.clone()))?;
    file.write_all(commit_hash.as_bytes()).map_err(|error| {
        CommandError::FileWriteError(format!(
            "Error al escribir en archivo {}: {}",
            currect_branch,
            error.to_string()
        ))
    })?;
    Ok(())
}

/// Opens file in .git/HEAD and returns the branch name
fn get_head_ref() -> Result<String, CommandError> {
    let Ok(mut head_file) = File::open(".git/HEAD") else {
        return Err(CommandError::FileOpenError(".git/HEAD".to_string()));
    };
    let mut head_content = String::new();
    head_file
        .read_to_string(&mut head_content)
        .map_err(|error| {
            CommandError::FileReadError(format!(
                "Error abriendo .git/HEAD: {:?}",
                error.to_string()
            ))
        })?;

    let Some((_, head_ref)) = head_content.split_once(" ") else {
        return Err(CommandError::FileReadError(
            "Error leyendo .git/HEAD".to_string(),
        ));
    };
    Ok(head_ref.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_invalid_arg() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args = ["-no".to_string()];
        match Commit::run_from(
            "commit",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => assert_eq!(error, CommandError::InvalidArguments),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_invalid_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "prueba2";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args = ["-m".to_string()];
        match Commit::run_from(
            "commit",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => assert_eq!(error, CommandError::CommitMessageNoValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_empty_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "q\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args = [];
        match Commit::run_from(
            "commit",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => assert_eq!(error, CommandError::CommitMessageEmptyValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_message_and_reuse() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args = [
            "-m".to_string(),
            "message".to_string(),
            "-C".to_string(),
            "hash todavía no se chequea".to_string(),
        ];
        match Commit::run_from(
            "commit",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => assert_eq!(error, CommandError::MessageAndReuseError),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_reuse_no_message() {
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "\n";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let mut logger = Logger::new(".git/logs").unwrap();

        let args = ["-m".to_string(), "message".to_string(), "-C".to_string()];
        match Commit::run_from(
            "commit",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        ) {
            Err(error) => assert_eq!(error, CommandError::ReuseMessageNoValue),
            Ok(_) => assert!(false),
        }
    }

    #[test]
    fn test_enter_message() {
        let input = "#Evitar\nMessage\nq\n";
        let expected = "Message".to_string();
        let mut stdin_mock = Cursor::new(input.as_bytes());

        match Commit::run_enter_message(&mut stdin_mock) {
            Err(error) => assert!(false, "{}", error),
            Ok(message) => assert_eq!(message, expected),
        }
    }

    /*
    Aclaración: El resto de las funciones son testeadas en tests/ porque necesitan de la existencia
    de un repositorio y de acciones previas como 'add'.
     */
}
