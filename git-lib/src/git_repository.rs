use std::{
    collections::HashMap,
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

use crate::{
    branch_manager::{get_current_branch_name, get_last_commit},
    changes_controller_components::{
        format::Format, long_format::LongFormat, short_format::ShortFormat,
    },
    command_errors::CommandError,
    config::Config,
    logger::Logger,
    objects::{
        author::Author,
        aux::get_name,
        blob::Blob,
        commit_object::{write_commit_tree_to_database, CommitObject},
        git_object::{self, GitObject, GitObjectTrait},
        last_commit::{get_commit_tree, is_in_last_commit},
        proto_object::ProtoObject,
        super_string::u8_vec_to_hex_string,
        tree::Tree,
    },
    objects_database,
    server_components::git_server::GitServer,
    staging_area::StagingArea,
};

pub struct GitRepository<'a> {
    path: String,
    logger: Logger,
    output: &'a mut dyn Write,
}

impl<'a> GitRepository<'a> {
    pub fn open(path: &str, output: &'a mut dyn Write) -> Result<GitRepository<'a>, CommandError> {
        if !Path::new(&format!("{}.git", path)).exists() {
            return Err(CommandError::NotGitRepository);
        }
        let logs_path = format!("{}.git/logs", path);
        Ok(GitRepository {
            path: path.to_string(),
            logger: Logger::new(&logs_path)?,
            output,
        })
    }

    pub fn init(
        path: &str,
        branch_name: &str,
        bare: bool,
        output: &'a mut dyn Write,
    ) -> Result<GitRepository<'a>, CommandError> {
        let logs_path = format!("{}/.git/logs", path);

        let mut repo = GitRepository {
            path: path.to_string(),
            logger: Logger::new(&logs_path)?,
            output,
        };
        repo.create_files_and_dirs(path, branch_name, bare)?;
        Ok(repo)
    }

    fn create_files_and_dirs(
        &mut self,
        path: &str,
        branch_name: &str,
        bare: bool,
    ) -> Result<(), CommandError> {
        self.create_dirs(path, bare)?;
        self.create_files(path, bare, branch_name)?;
        let output_text = format!("Initialized empty Git repository in {}", path);
        let _ = writeln!(self.output, "{}", output_text);
        Ok(())
    }

    fn create_dirs(&self, path: &str, bare: bool) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_err() {
            return Err(CommandError::DirectoryCreationError(path.to_string()));
        }
        let git_path = if bare {
            path.to_string()
        } else {
            format!("{}/.git", path.to_string())
        };
        self.create_dir(&git_path, "objects".to_string())?;
        self.create_dir(&git_path, "objects/info".to_string())?;
        self.create_dir(&git_path, "objects/pack".to_string())?;
        self.create_dir(&git_path, "refs".to_string())?;
        self.create_dir(&git_path, "refs/tags".to_string())?;
        self.create_dir(&git_path, "refs/heads".to_string())?;
        self.create_dir(&git_path, "branches".to_string())?;
        Ok(())
    }

    fn create_dir(&self, path: &String, name: String) -> Result<(), CommandError> {
        let path_complete = format!("{}/{}", path, name);
        if fs::create_dir_all(&path_complete).is_ok() {
            Ok(())
        } else {
            Err(CommandError::DirectoryCreationError(path_complete))
        }
    }

    fn create_files(&self, path: &str, bare: bool, branch_name: &str) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_err() {
            return Err(CommandError::DirectoryCreationError(path.to_string()));
        }
        let path_aux = if bare {
            path.to_string()
        } else {
            format!("{}/.git", path)
        };
        self.create_file(&path_aux, "HEAD".to_string(), branch_name)?;
        Ok(())
    }

    fn create_file(&self, path: &str, name: String, branch_name: &str) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_ok() {
            let path_complete = format!("{}/{}", path, name);
            match File::create(&path_complete) {
                Ok(mut archivo) => {
                    let texto = format!("ref: refs/heads/{}", branch_name.to_string());
                    let _: Result<(), CommandError> = match archivo.write_all(texto.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(CommandError::FileWriteError(err.to_string())),
                    };
                }
                Err(err) => return Err(CommandError::FileCreationError(err.to_string())),
            };
        } else {
            return Err(CommandError::DirectoryCreationError(path.to_string()));
        }
        Ok(())
    }

    pub fn hash_object(&mut self, mut object: GitObject, write: bool) -> Result<(), CommandError> {
        let hex_string = u8_vec_to_hex_string(&mut object.get_hash()?);
        if write {
            objects_database::write(&mut self.logger, &mut object)?;
            self.logger
                .log(&format!("Writen object to database in {:?}", hex_string));
        }
        let _ = writeln!(self.output, "{}", hex_string);

        Ok(())
    }

    pub fn add(&mut self, pathspecs: Vec<String>) -> Result<(), CommandError> {
        let last_commit = &get_commit_tree(&mut self.logger)?;
        let mut staging_area = StagingArea::open()?;
        let mut pathspecs_clone: Vec<String> = pathspecs.clone();
        let mut position = 0;
        for pathspec in &pathspecs {
            if !Path::new(pathspec).exists() {
                if !self.is_in_last_commit(pathspec, last_commit) {
                    return Err(CommandError::FileOpenError(format!(
                        "No existe el archivo o directorio: {:?}",
                        pathspec
                    )));
                }
                staging_area.remove(pathspec);
                pathspecs_clone.remove(position);
                continue;
            }
            position += 1;
        }

        for pathspec in pathspecs_clone.iter() {
            self.add_path(pathspec, &mut staging_area)?
        }
        staging_area.save()?;
        Ok(())
    }

    fn add_path(&mut self, path: &str, staging_area: &mut StagingArea) -> Result<(), CommandError> {
        let path = Path::new(path);
        let path_str = &Self::get_path_str(path)?;

        if path.is_file() {
            self.add_file(path_str, staging_area)?;
            return Ok(());
        } else {
            self.add_dir(path_str, staging_area)?;
        }
        Ok(())
    }
    fn add_dir(
        &mut self,
        path_str: &String,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let read_dir = self.read_dir(path_str)?;
        for entry in read_dir {
            match entry {
                Ok(entry) => self.try_run_for_path(entry, staging_area)?,
                Err(error) => {
                    self.log(&format!("Error in entry: {:?}", error));
                    return Err(CommandError::FileOpenError(error.to_string()));
                }
            }
        }
        Ok(())
    }

    fn should_ignore(&self, path_str: &str) -> bool {
        path_str == "./.git"
    }

    fn try_run_for_path(
        &mut self,
        entry: DirEntry,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let path = entry.path();
        let Some(path_str) = path.to_str() else {
            return Err(CommandError::FileOpenError(
                "No se pudo convertir el path a str".to_string(),
            ));
        };
        if self.should_ignore(path_str) {
            return Ok(());
        }
        self.log(&format!("entry: {:?}", path_str));
        self.add_path(path_str, staging_area)?;
        Ok(())
    }

    fn read_dir(&self, path_str: &String) -> Result<ReadDir, CommandError> {
        match fs::read_dir(path_str) {
            Ok(read_dir) => Ok(read_dir),
            Err(error) => Err(CommandError::FileOpenError(error.to_string())),
        }
    }

    fn get_path_str(path: &Path) -> Result<String, CommandError> {
        let Some(path_str) = path.to_str() else {
            return Err(CommandError::FileOpenError(
                "No se pudo convertir el path a str".to_string(),
            ));
        };
        Ok(path_str.to_string())
    }

    pub fn add_file(
        &mut self,
        path: &str,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let blob = Blob::new_from_path(path.to_string())?;
        let mut git_object: GitObject = Box::new(blob);
        let hex_str = objects_database::write(&mut self.logger, &mut git_object)?;
        staging_area.add(path, &hex_str);
        Ok(())
    }

    fn is_in_last_commit(&mut self, path: &str, commit_tree: &Option<Tree>) -> bool {
        if let Some(tree) = commit_tree {
            return tree.has_blob_from_path(path, &mut self.logger);
        }
        false
    }

    pub fn display_type_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_type_from_hash(self.output, hash, &mut self.logger)
    }

    pub fn display_size_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_size_from_hash(self.output, hash, &mut self.logger)
    }

    pub fn display_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_from_hash(self.output, hash, &mut self.logger)
    }

    pub fn commit_files(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        let mut staging_area = StagingArea::open()?;
        self.update_staging_area_files(&files, &mut staging_area)?;

        self.commit_priv(
            message,
            &mut staging_area,
            files,
            dry_run,
            reuse_commit_info,
            quiet,
        )
    }

    pub fn commit_all(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        let mut staging_area = StagingArea::open()?;
        self.run_all_config(&mut staging_area)?;

        self.commit_priv(
            message,
            &mut staging_area,
            files,
            dry_run,
            reuse_commit_info,
            quiet,
        )
    }

    pub fn commit(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        self.log("Running commit");
        let mut staging_area = StagingArea::open()?;
        self.log("StagingArea opened");
        self.commit_priv(
            message,
            &mut staging_area,
            files,
            dry_run,
            reuse_commit_info,
            quiet,
        )
    }

    /// Si se han introducido paths como argumentos del comando, se eliminan los cambios
    /// guardados en el Staging Area y se agregan los nuevos.\
    /// Estos archivos deben ser reconocidos por git.
    fn update_staging_area_files(
        &mut self,
        files: &Vec<String>,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        self.log("Running pathspec configuration");
        let staging_area_files = staging_area.get_files();

        for path in files.iter() {
            if !Path::new(path).exists() {
                staging_area.remove(path);
            }
            if !is_untracked(path, &mut self.logger, &staging_area_files)? {
                self.add_file(path, staging_area)?;
            } else {
                return Err(CommandError::UntrackedError(path.to_owned()));
            }
        }
        staging_area.save()?;

        Ok(())
    }

    /// Guarda en el staging area todos los archivos modificados y elimina los borrados.\
    /// Los archivos untracked no se guardan.
    fn run_all_config(&mut self, staging_area: &mut StagingArea) -> Result<(), CommandError> {
        self.log("Running 'all' configuration\n");
        let files = &staging_area.get_files();
        for (path, _) in files {
            if !Path::new(&path).exists() {
                staging_area.remove(&path);
            }
        }
        staging_area.remove_changes(&mut self.logger)?;
        save_entries("./", staging_area, &mut self.logger, files)?;
        staging_area.save()?;
        Ok(())
    }

    /// Ejecuta la creación del Commit.
    fn commit_priv(
        &mut self,
        message: String,
        staging_area: &mut StagingArea,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        if !staging_area.has_changes(&mut self.logger)? {
            self.logger.log("Nothing to commit");
            self.status_long_format(true)?;
            return Ok(());
        }

        let last_commit_hash = get_last_commit()?;

        let mut parents: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            parents.push(padre);
        }

        self.log("Creating Index tree");

        let mut staged_tree = {
            if files.is_empty() {
                staging_area.get_working_tree_staged(&mut self.logger)?
            } else {
                staging_area.get_working_tree_staged_bis(&mut self.logger, files.clone())?
            }
        };

        self.log("Index tree created");

        let commit: CommitObject =
            self.get_commit(&message, parents, staged_tree.to_owned(), reuse_commit_info)?;
        self.log("get_commit");

        let mut git_object: GitObject = Box::new(commit);

        if !dry_run {
            write_commit_tree_to_database(&mut staged_tree, &mut self.logger)?;
            let commit_hash = objects_database::write(&mut self.logger, &mut git_object)?;
            self.logger
                .log(&format!("Commit object saved in database {}", commit_hash));
            self.logger
                .log(&format!("Updating last commit to {}", commit_hash));

            update_last_commit(&commit_hash)?;
            self.logger.log("Last commit updated");
        }

        if !quiet {
            self.status_long_format(true)?;
        }

        Ok(())
    }

    /// Obtiene el objeto Commit y lo devuelve.
    fn get_commit(
        &mut self,
        message: &str,
        parents: Vec<String>,
        staged_tree: Tree,
        reuse_commit_info: Option<String>,
    ) -> Result<CommitObject, CommandError> {
        let commit: CommitObject = {
            if let Some(commit_hash) = &reuse_commit_info {
                self.get_reused_commit(commit_hash.to_string(), parents, staged_tree)?
            } else {
                self.create_new_commit(message.to_owned(), parents, staged_tree)?
            }
        };

        self.log("Commit object created");
        Ok(commit)
    }

    /// Crea un nuevo objeto Commit a partir de la información pasada.
    fn create_new_commit(
        &mut self,
        message: String,
        parents: Vec<String>,
        staged_tree: Tree,
    ) -> Result<CommitObject, CommandError> {
        let config = Config::open(&self.path)?;
        self.log("Config opened");
        let Some(author_email) = config.get("user", "email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(author_name) = config.get("user", "name") else {
            return Err(CommandError::UserConfigurationError);
        };

        let author = Author::new(author_name, author_email);
        let commiter = Author::new(author_name, author_email);
        self.log("Author and committr created");

        let datetime: DateTime<Local> = Local::now();
        let timestamp = datetime.timestamp();
        let offset = datetime.offset().local_minus_utc() / 60;
        self.log(&format!("offset: {}", offset));
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
        &mut self,
        commit_hash: String,
        parents: Vec<String>,
        staged_tree: Tree,
    ) -> Result<CommitObject, CommandError> {
        let mut other_commit = objects_database::read_object(&commit_hash, &mut self.logger)?;
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

    pub fn status_long_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let branch = get_current_branch_name()?;
        let long_format = LongFormat;
        long_format.show(&mut self.logger, &mut self.output, &branch, commit_output)
    }

    pub fn status_short_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let branch = get_current_branch_name()?;
        let short_format = ShortFormat;
        short_format.show(&mut self.logger, &mut self.output, &branch, commit_output)
    }

    pub fn config(&self) -> Result<Config, CommandError> {
        Config::open(&self.path)
    }

    pub fn update_remote(&self, url: String) -> Result<(), CommandError> {
        let mut config = self.config()?;
        config.insert("remote \"origin\"", "url", &url);
        config.save()?;
        Ok(())
    }

    pub fn fetch(&mut self) -> Result<(), CommandError> {
        let config = self.config()?;

        let Some(url) = config.get("remote \"origin\"", "url") else {
            return Err(CommandError::NoRemoteUrl);
        };

        let Some((address, repository_path)) = url.split_once('/') else {
            return Err(CommandError::InvalidConfigFile);
        };

        let (repository_url, _repository_port) = {
            match address.split_once(':') {
                Some((repository_url, port)) => (repository_url, Some(port)),
                None => (address, None),
            }
        };

        let mut server = GitServer::connect_to(&address)?;
        update_remote_branches(&mut server, repository_path, repository_url, &self.path)?;
        self.fetch_and_save_objects(&mut server)?;
        Ok(())
    }

    fn fetch_and_save_objects(&mut self, server: &mut GitServer) -> Result<(), CommandError> {
        let wants = remote_branches(&self.path)?.into_values().collect();
        let haves = local_branches(&self.path)?.into_values().collect();
        let objects_decompressed_data = server.fetch_objects(wants, haves)?;
        for (obj_type, len, content) in objects_decompressed_data {
            self.log(&format!(
                "Saving object of type {} and len {}, with data {:?}",
                obj_type,
                len,
                String::from_utf8_lossy(&content)
            ));
            let mut git_object: GitObject =
                Box::new(ProtoObject::new(content, len, obj_type.to_string()));
            objects_database::write_to(
                &mut Logger::new_dummy(),
                &mut git_object,
                &format!("{}/", self.path),
            )?;
        }
        Ok(())
    }

    pub fn log(&mut self, content: &str) {
        self.logger.log(content);
    }

    pub fn pull(&mut self) -> Result<(), CommandError> {
        self.fetch()?;
        self.merge()?;
        Ok(())
    }

    fn merge(&self) -> Result<(), CommandError> {
        todo!()
    }
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
        if entry_path.is_dir() {
            save_entries(&entry_name, staging_area, logger, files)?;
            return Ok(());
        } else {
            let blob = Blob::new_from_path(entry_name.to_string())?;
            let path = &entry_name[2..];
            if !is_untracked(path, logger, files)? {
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
pub fn get_head_ref() -> Result<String, CommandError> {
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

pub fn local_branches(base_path: &str) -> Result<HashMap<String, String>, CommandError> {
    let mut branches = HashMap::<String, String>::new();
    let branches_path = format!("{}/.git/refs/heads/", base_path);
    let paths = fs::read_dir(branches_path).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error leyendo directorio de branches: {}",
            error.to_string()
        ))
    })?;
    for path in paths {
        let path = path.map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let file_name = &path.file_name();
        let Some(file_name) = file_name.to_str() else {
            return Err(CommandError::FileReadError(
                "Error leyendo directorio de branches".to_string(),
            ));
        };
        let mut file = fs::File::open(path.path()).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let mut sha1 = String::new();
        file.read_to_string(&mut sha1).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        branches.insert(file_name.to_string(), sha1);
    }
    Ok(branches)
}

fn remote_branches(base_path: &str) -> Result<HashMap<String, String>, CommandError> {
    let mut branches = HashMap::<String, String>::new();
    let branches_path = format!("{}/.git/refs/remotes/origin/", base_path);
    let paths = fs::read_dir(branches_path).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error leyendo directorio de branches: {}",
            error.to_string()
        ))
    })?;
    for path in paths {
        let path = path.map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let file_name = &path.file_name();
        let Some(file_name) = file_name.to_str() else {
            return Err(CommandError::FileReadError(
                "Error leyendo directorio de branches".to_string(),
            ));
        };
        let mut file = fs::File::open(path.path()).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let mut sha1 = String::new();
        file.read_to_string(&mut sha1).map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        branches.insert(file_name.to_string(), sha1);
    }
    Ok(branches)
}

fn update_remote_branches(
    server: &mut GitServer,
    repository_path: &str,
    repository_url: &str,
    base_path: &str,
) -> Result<(), CommandError> {
    let (_head_branch, branch_remote_refs) =
        server.explore_repository(&("/".to_owned() + repository_path), repository_url)?;
    Ok(for (sha1, mut ref_path) in branch_remote_refs {
        ref_path.replace_range(0..11, "");
        update_ref(&base_path, &sha1, &ref_path)?;
    })
}

fn update_ref(base_path: &str, sha1: &str, ref_name: &str) -> Result<(), CommandError> {
    let dir_path = format!("{}/.git/refs/remotes/origin/", base_path);
    let file_path = dir_path.to_owned() + ref_name;

    fs::create_dir_all(dir_path).map_err(|error| {
        CommandError::DirectoryCreationError(format!(
            "Error creando directorio de refs: {}",
            error.to_string()
        ))
    })?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&file_path)
        .map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error guardando ref en {}: {}",
                file_path,
                &error.to_string()
            ))
        })?;
    file.write_all(sha1.as_bytes()).map_err(|error| {
        CommandError::FileWriteError("Error guardando ref:".to_string() + &error.to_string())
    })?;
    Ok(())
}
