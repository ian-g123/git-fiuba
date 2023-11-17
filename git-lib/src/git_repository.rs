use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

use crate::{
    changes_controller_components::{
        changes_controller::ChangesController,
        changes_types::{self, ChangeType},
        commit_format::CommitFormat,
        format::Format,
        long_format::{sort_hashmap_and_filter_unmodified, LongFormat},
        short_format::ShortFormat,
        working_tree::{build_working_tree, get_path_name},
    },
    command_errors::CommandError,
    config::Config,
    diff_components::merge::merge_content,
    join_paths,
    logger::{self, Logger},
    objects::{
        author::Author,
        blob::Blob,
        commit_object::{
            sort_commits_descending_date, write_commit_tree_to_database, CommitObject,
        },
        git_object::{self, GitObject, GitObjectTrait},
        mode::Mode,
        proto_object::ProtoObject,
        tree::Tree,
    },
    objects_database::ObjectsDatabase,
    server_components::git_server::GitServer,
    server_components::{
        history_analyzer::{get_analysis, get_parents_hash_map, rebuild_commits_tree},
        packfile_functions::make_packfile,
        packfile_object_type::PackfileObjectType,
    },
    staging_area::StagingArea,
    utils::{aux::get_name, super_string::u8_vec_to_hex_string},
};

pub struct GitRepository<'a> {
    git_path: String,
    working_dir_path: String,
    logger: Logger,
    output: &'a mut dyn Write,
    bare: bool,
}

impl<'a> GitRepository<'a> {
    pub fn open(path: &str, output: &'a mut dyn Write) -> Result<GitRepository<'a>, CommandError> {
        let tentative_git_path = join_paths!(path, ".git").ok_or(
            CommandError::DirectoryCreationError("Error abriendo directorio .git".to_string()),
        )?;
        let (git_path, logger, bare) = if Path::new(&tentative_git_path).exists() {
            let logs_path = &join_paths!(tentative_git_path, "logs").ok_or(
                CommandError::DirectoryCreationError("Error creando archivo .git/logs".to_string()),
            )?;
            (tentative_git_path, Logger::new(&logs_path)?, false)
        } else {
            (path.to_string(), Logger::new_dummy(), true)
        };
        if !Path::new(&join_paths!(git_path, "objects").ok_or(
            CommandError::DirectoryCreationError("Error abriendo directorio .git".to_string()),
        )?)
        .exists()
        {
            println!(
                "No se encontro la carpeta {:?}",
                join_paths!(git_path, "objects")
            );
            return Err(CommandError::NotGitRepository);
        }

        Ok(GitRepository {
            git_path: git_path,
            working_dir_path: path.to_string(),
            logger,
            output,
            bare,
        })
    }

    pub fn init(
        path: &str,
        branch_name: &str,
        bare: bool,
        output: &'a mut dyn Write,
    ) -> Result<GitRepository<'a>, CommandError> {
        let (git_path, logger) = if bare {
            (path.to_string(), Logger::new_dummy())
        } else {
            let tentative_git_path = join_paths!(path, ".git").ok_or(
                CommandError::DirectoryCreationError("Error creando directorio .git".to_string()),
            )?;
            let logs_path = &join_paths!(tentative_git_path, "logs").ok_or(
                CommandError::DirectoryCreationError("Error creando archivo .git/logs".to_string()),
            )?;
            (tentative_git_path, Logger::new(&logs_path)?)
        };

        let mut repo = GitRepository {
            git_path,
            working_dir_path: path.to_string(),
            logger,
            output,
            bare,
        };
        repo.create_files_and_dirs(branch_name)?;
        Ok(repo)
    }

    fn create_files_and_dirs(&mut self, branch_name: &str) -> Result<(), CommandError> {
        self.create_dirs()?;
        self.create_files(branch_name)?;
        let output_text = format!(
            "Initialized empty Git repository in {}",
            &self.working_dir_path
        );
        let _ = writeln!(self.output, "{}", output_text);
        Ok(())
    }

    fn create_dirs(&self) -> Result<(), CommandError> {
        if fs::create_dir_all(&self.git_path).is_err() {
            return Err(CommandError::DirectoryCreationError(
                self.git_path.to_owned(),
            ));
        }
        // let git_path = if bare {
        //     path.to_string()
        // } else {
        //     join_paths!(path.to_string(), ".git").ok_or(CommandError::DirectoryCreationError(
        //         "Error creando arc".to_string(),
        //     ))?
        // };
        self.create_dir(&self.git_path, "objects".to_string())?;
        self.create_dir(&self.git_path, "objects/info".to_string())?;
        self.create_dir(&self.git_path, "objects/pack".to_string())?;
        self.create_dir(&self.git_path, "refs".to_string())?;
        self.create_dir(&self.git_path, "refs/tags".to_string())?;
        self.create_dir(&self.git_path, "refs/heads".to_string())?;
        self.create_dir(&self.git_path, "branches".to_string())?;
        Ok(())
    }

    fn create_dir(&self, path: &str, name: String) -> Result<(), CommandError> {
        let path_complete = join_paths!(path, name).ok_or(CommandError::DirectoryCreationError(
            "Error creando directorio".to_string(),
        ))?;
        if fs::create_dir_all(&path_complete).is_ok() {
            Ok(())
        } else {
            Err(CommandError::DirectoryCreationError(path_complete))
        }
    }

    fn create_files(&self, branch_name: &str) -> Result<(), CommandError> {
        let name = "HEAD".to_string();
        let branch_name = branch_name;
        if fs::create_dir_all(&self.git_path).is_ok() {
            let path_complete = join_paths!(&self.git_path, name).ok_or(
                CommandError::FileCreationError("Error creando un archivo".to_string()),
            )?;
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
            return Err(CommandError::DirectoryCreationError(
                self.git_path.to_string(),
            ));
        }
        Ok(())
    }

    pub fn hash_object(&mut self, mut object: GitObject, write: bool) -> Result<(), CommandError> {
        let hex_string = u8_vec_to_hex_string(&mut object.get_hash()?);
        if write {
            self.db()?.write(&mut object, false, &mut self.logger)?;
            self.logger
                .log(&format!("Writen object to database in {:?}", hex_string));
        }
        let _ = writeln!(self.output, "{}", hex_string);

        Ok(())
    }

    pub fn add(&mut self, pathspecs: Vec<String>) -> Result<(), CommandError> {
        let last_commit = &self.get_last_commit_tree()?;
        let mut staging_area = StagingArea::open(&self.git_path)?;
        let mut pathspecs_clone: Vec<String> = pathspecs.clone();
        let mut position = 0;
        for pathspec in &pathspecs {
            if !Path::new(pathspec).exists() {
                if !self.is_in_last_commit_from_path(pathspec, last_commit) {
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

    pub fn rm(
        &mut self,
        pathspecs: Vec<String>,
        force: bool,
        recursive: bool,
    ) -> Result<(), CommandError> {
        if !self.working_dir_path.is_empty() {
            todo!("rm en subdirectorios")
        }
        for pathspec in &pathspecs {
            if !Path::new(pathspec).exists() {
                return Err(CommandError::FileOpenError(format!(
                    "No existe el archivo o directorio: {:?}",
                    pathspec
                )));
            }
            self.verifies_directory(pathspec, recursive)?;
        }

        let mut staging_area = StagingArea::open(&self.git_path)?;
        for pathspec in &pathspecs {
            self.run_for_path_rm(pathspec, force, &mut staging_area)?
        }
        staging_area.save()?;
        Ok(())
    }

    fn verifies_directory(&self, pathspec: &String, recursive: bool) -> Result<(), CommandError> {
        let path = Path::new(pathspec);
        if path.is_dir() && !recursive {
            return Err(CommandError::NotRecursive(pathspec.clone()));
        }
        Ok(())
    }

    fn run_for_path_rm(
        &mut self,
        path: &str,
        force: bool,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let path = Path::new(path);
        let path_str = &get_path_str(path.to_path_buf())?;

        if path.is_file() {
            self.run_for_file_rm(path_str, force, staging_area)?;
            return Ok(());
        } else {
            self.run_for_dir_rm(path_str, force, staging_area)?;
        }
        Ok(())
    }

    fn run_for_file_rm(
        &mut self,
        path: &str,
        force: bool,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        if !force {
            self.verifies_version_of_index(path, staging_area)?;
        }

        staging_area.remove_from_stagin_area(path, &mut self.logger)?;

        fs::remove_file(path).map_err(|_| {
            CommandError::FileOpenError(format!("No existe el archivo: {:?}", path))
        })?;

        Ok(())
    }

    /// Verifica que la versión del commit anterior sea la misma. Los archivos que se eliminan
    /// deben ser idénticos al últmo commit, y no se pueden haber agregado nuevas versiones
    /// de este al index."
    fn verifies_version_of_index(
        &mut self,
        path: &str,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let mut tree_last_commit = match self.get_last_commit_tree()? {
            Some(tree_last_commit_box) => tree_last_commit_box,
            None => return Err(CommandError::RmFromStagingAreaError(path.to_string())),
        };
        let actual_hash = staging_area.get_hash_from_path(path)?;
        let (exist, name_blob) =
            tree_last_commit.has_blob_from_hash(&actual_hash, &mut self.logger)?;
        if exist && name_blob != get_name(path)? {
            return Err(CommandError::RmFromStagingAreaError(path.to_string()));
        }
        return Ok(());
    }

    pub fn remove_cached(&mut self, pathspecs: Vec<String>) -> Result<(), CommandError> {
        let mut tree_last_commit = match self.get_last_commit_tree() {
            Ok(Some(tree_last_commit_box)) => tree_last_commit_box,
            _ => {
                return Err(CommandError::RmFromStagingAreaError(
                    "Error abriendo el último commit".to_string(),
                ))
            }
        };

        let mut staging_area = match StagingArea::open(&self.git_path) {
            Ok(staging_area) => staging_area,
            _ => {
                return Err(CommandError::RmFromStagingAreaError(
                    "Error abriendo el último commit".to_string(),
                ))
            }
        };

        for relative_path in &pathspecs {
            if staging_area.is_in_staging_area(&relative_path.to_string()) {
                let git_object_option = tree_last_commit.get_object_from_path(relative_path);
                if let Some(mut git_object) = git_object_option {
                    let actual_hash_lc = match git_object.get_hash_string() {
                        Ok(hash) => hash,
                        _ => {
                            return Err(CommandError::RmFromStagingAreaError(
                                "Error abriendo el último commit".to_string(),
                            ))
                        }
                    };
                    staging_area.add(relative_path, &actual_hash_lc);
                } else {
                    staging_area.remove(relative_path);
                }
            }
        }
        if staging_area.save().is_err() {
            println!("Error al guardar el staging area");
        };
        Ok(())
    }

    fn run_for_dir_rm(
        &mut self,
        path_str: &String,
        force: bool,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        // let read_dir = self.read_dir(&join_paths!(self.working_dir_path, path_str).ok_or(
        //     CommandError::DirectoryCreationError("Error abriendo directorio".to_string()),
        // )?)?;
        let read_dir = self.read_dir(path_str)?;

        for entry in read_dir {
            match entry {
                Ok(entry) => self.try_run_for_path_rm(entry, force, staging_area)?,
                Err(error) => {
                    self.logger.log(&format!("Error en entry: {:?}", error));
                    return Err(CommandError::FileOpenError(error.to_string()));
                }
            }
        }
        Ok(())
    }

    fn try_run_for_path_rm(
        &mut self,
        entry: DirEntry,
        force: bool,
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

        self.logger.log(&format!("entry: {:?}", path_str));
        self.run_for_path_rm(path_str, force, staging_area)?;
        Ok(())
    }

    fn add_path(&mut self, path: &str, staging_area: &mut StagingArea) -> Result<(), CommandError> {
        let path = Path::new(path);
        let path_str = &Self::get_path_str(path)?;
        let path_str = join_paths!(self.working_dir_path, path_str).ok_or(
            CommandError::DirectoryCreationError("Error abriendo directorio".to_string()),
        )?;

        if path.is_file() {
            self.add_file(&path_str, staging_area)?;
            return Ok(());
        } else {
            self.add_dir(&path_str, staging_area)?;
        }
        Ok(())
    }
    fn add_dir(
        &mut self,
        path_str: &str,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let entries = self.read_dir(&path_str.to_string())?;
        for entry in entries {
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
        path_str.starts_with("./.git") || path_str.starts_with(".git")
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
        let path_str_r = if self.working_dir_path.is_empty() {
            path_str.to_string()
        } else {
            path_str[(self.working_dir_path.len() + 1)..].to_string()
        };
        if self.should_ignore(&path_str_r) {
            return Ok(());
        }
        self.log(&format!("entry: {:?}", path_str_r));
        self.add_path(&path_str_r, staging_area)?;
        Ok(())
    }

    fn read_dir(&mut self, path_str: &String) -> Result<ReadDir, CommandError> {
        match fs::read_dir(path_str) {
            Ok(read_dir) => Ok(read_dir),
            Err(error) => {
                self.logger.log(&format!(
                    "Error en read_dir: {error} desde {:?}",
                    env::current_dir()
                ));
                Err(CommandError::FileOpenError(error.to_string()))
            }
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
        let hex_str = self.db()?.write(&mut git_object, false, &mut self.logger)?;
        self.log(&format!("File {} (hash: {}) added to index", path, hex_str));
        staging_area.add(path, &hex_str);
        Ok(())
    }

    fn is_in_last_commit_from_path(&mut self, path: &str, commit_tree: &Option<Tree>) -> bool {
        if let Some(tree) = commit_tree {
            return tree.has_blob_from_path(path, &mut self.logger);
        }
        false
    }

    pub fn is_in_last_commit_from_hash(
        &mut self,
        blob_hash: String,
    ) -> Result<(bool, String), CommandError> {
        if let Some(mut tree) = self.get_last_commit_tree()? {
            return Ok(tree.has_blob_from_hash(&blob_hash, &mut self.logger)?);
        }
        Ok((false, "".to_string()))
    }

    pub fn display_type_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_type_from_hash(&self.db()?, self.output, hash, &mut self.logger)
    }

    pub fn display_size_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_size_from_hash(&self.db()?, self.output, hash, &mut self.logger)
    }

    pub fn display_from_hash(&mut self, hash: &str) -> Result<(), CommandError> {
        git_object::display_from_hash(&self.db()?, self.output, hash, &mut self.logger)
    }

    pub fn commit_files(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        let mut staging_area = StagingArea::open(&self.git_path)?;
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
        let mut staging_area = StagingArea::open(&self.git_path)?;
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
        let mut staging_area = StagingArea::open(&self.git_path)?;
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
        self.log("staging area files");

        for path in files.iter() {
            self.log(&format!("Updating: {}", path));
            if !Path::new(path).exists() {
                self.log(&format!("Removed from index: {}", path));

                staging_area.remove(path);
            } else if !self.is_untracked(path, &staging_area)? {
                self.log(&format!("It's untracked: {}", path));

                self.add_file(path, staging_area)?;
            } else {
                return Err(CommandError::UntrackedError(path.to_owned()));
            }
        }
        self.log("Saving staging area changes");

        staging_area.save()?;
        self.log("Finished running pathspec configuration");

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

        self.log("Saving entries\n");

        self.save_entries("./", staging_area, files)?;
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
        if self.is_merge()? && staging_area.has_conflicts() {
            return Err(CommandError::MergeConflictsCommit);
        }

        let last_commit_tree = self.get_last_commit_tree()?;
        self.log("Checking if index is empty");
        if !staging_area.has_changes(&self.db()?, &last_commit_tree, &mut self.logger)? {
            self.logger.log("Nothing to commit");
            self.status_long_format(true)?;
            return Ok(());
        }

        let last_commit_hash = self.get_last_commit_hash()?;

        let mut parents: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            parents.push(padre);
        }

        let mut staged_tree = {
            if files.is_empty() {
                self.log("is_empty");
                staging_area.get_working_tree_staged(&mut self.logger)?
            } else {
                self.log("files isnt empty");

                staging_area.get_working_tree_staged_bis(
                    &last_commit_tree,
                    &mut self.logger,
                    files.clone(),
                )?
            }
        };
        self.log("BBBBBB");

        let commit: CommitObject =
            self.get_commit(&message, parents, staged_tree.to_owned(), reuse_commit_info)?;

        let mut git_object: GitObject = Box::new(commit);
        let current_branch = &self.get_current_branch_name()?;
        if !dry_run {
            write_commit_tree_to_database(&mut self.db()?, &mut staged_tree, &mut self.logger)?;
            let commit_hash = self.db()?.write(&mut git_object, false, &mut self.logger)?;
            update_last_commit(&commit_hash)?;

            self.logger.log("Last commit updated");
            if !quiet {
                CommitFormat::show(
                    &staging_area,
                    &self.db()?,
                    &mut self.logger,
                    last_commit_tree,
                    &commit_hash,
                    current_branch,
                    &message,
                    &mut self.output,
                    &self.git_path,
                    &self.working_dir_path,
                )?;
            }
        } else {
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
        Ok(commit)
    }

    /// Crea un nuevo objeto Commit a partir de la información pasada.
    fn create_new_commit(
        &mut self,
        message: String,
        parents: Vec<String>,
        staged_tree: Tree,
    ) -> Result<CommitObject, CommandError> {
        let config = Config::open(&self.git_path)?;
        let Some(author_email) = config.get("user", "email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(author_name) = config.get("user", "name") else {
            return Err(CommandError::UserConfigurationError);
        };
        let author = Author::new(author_name, author_email);
        let commiter = Author::new(author_name, author_email);
        let datetime: DateTime<Local> = Local::now();
        let timestamp = datetime.timestamp();
        let offset = datetime.offset().local_minus_utc() / 60;
        let commit = CommitObject::new_from_tree(
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
        let mut other_commit = self.db()?.read_object(&commit_hash, &mut self.logger)?;
        let hash_commit = other_commit.get_hash()?;
        if let Some((message, author, committer, timestamp, offset)) =
            other_commit.get_info_commit()
        {
            let commit = CommitObject::new_from_tree(
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

    fn is_merge(&self) -> Result<bool, CommandError> {
        let path = join_paths!(self.git_path, "MERGE_HEAD").ok_or(CommandError::FileNameError)?;
        if Path::new(&path).exists() {
            return Ok(true);
        }
        Ok(false)
    }

    fn get_commits_ahead_and_behind_remote(
        &mut self,
        branch: &str,
    ) -> Result<(bool, usize, usize), CommandError> {
        let remote_branches = match self.remote_branches() {
            Ok(remote_branches) => {
                if remote_branches.is_empty() {
                    return Ok((false, 0, 0));
                }
                remote_branches
            }
            Err(_) => {
                return Ok((false, 0, 0));
            }
        };
        let commit_head = self.get_last_commit_hash()?;
        let commit_remote = remote_branches.get(branch);
        let common_hash = match (commit_head.clone(), commit_remote) {
            (Some(head), Some(remote)) => {
                let (mut common, _, _) = self.get_common_ansestor(remote, &head)?;
                common.get_hash_string()?
            }
            _ => "".to_string(),
        };
        let (ahead, behind) =
            self.count_commits_ahead_and_behind(commit_remote, commit_head, &common_hash)?;
        Ok((true, ahead, behind))
    }

    pub fn status_long_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let branch = self.get_current_branch_name()?;
        let long_format = LongFormat;
        let last_commit_tree = self.get_last_commit_tree()?;
        let merge = self.is_merge()?;
        let diverge_info = self.get_commits_ahead_and_behind_remote(&branch)?;
        let index = self.staging_area()?;
        long_format.show(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            last_commit_tree,
            &mut self.logger,
            &mut self.output,
            &branch,
            commit_output,
            merge,
            diverge_info,
            &index,
        )
    }

    pub fn status_short_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let branch = self.get_current_branch_name()?;
        let short_format = ShortFormat;
        let last_commit_tree = self.get_last_commit_tree()?;
        let merge = self.is_merge()?;
        let diverge_info = self.get_commits_ahead_and_behind_remote(&branch)?;
        let index = self.staging_area()?;

        short_format.show(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            last_commit_tree,
            &mut self.logger,
            &mut self.output,
            &branch,
            commit_output,
            merge,
            diverge_info,
            &index,
        )
    }

    fn count_commits_ahead_and_behind(
        &mut self,
        commit_destin_str: Option<&String>,
        commit_head_str: Option<String>,
        common_hash: &str,
    ) -> Result<(usize, usize), CommandError> {
        let mut head_branch_tips: Vec<String> = Vec::new();
        if let Some(head) = commit_head_str {
            if head != common_hash {
                head_branch_tips.push(head.to_string())
            }
        }
        let mut destin_branch_tips: Vec<String> = Vec::new();

        if let Some(destin) = commit_destin_str {
            if destin != common_hash {
                destin_branch_tips.push(destin.to_string())
            }
        }
        let mut ahead = 0;
        let mut behind = 0;
        let db = self.db()?;
        loop {
            if head_branch_tips.is_empty() && destin_branch_tips.is_empty() {
                break;
            }
            if !head_branch_tips.is_empty() {
                ahead += head_branch_tips.len();
            }
            if !destin_branch_tips.is_empty() {
                behind += destin_branch_tips.len();
            }

            self.get_new_tips(&mut head_branch_tips, &common_hash, &db)?;
            self.get_new_tips(&mut destin_branch_tips, &common_hash, &db)?;
        }
        Ok((ahead, behind))
    }

    fn get_new_tips(
        &mut self,
        branch_tips: &mut Vec<String>,
        common: &str,
        db: &ObjectsDatabase,
    ) -> Result<(), CommandError> {
        let mut new_branch_tips = Vec::<String>::new();
        for tip in branch_tips.iter() {
            let commit = db
                .read_object(&tip, &mut self.logger)?
                .as_mut_commit()
                .ok_or(CommandError::FailedToFindCommonAncestor)?
                .to_owned();
            let parents_hash = commit.get_parents();
            for parent_hash in parents_hash {
                if !new_branch_tips.contains(&parent_hash) && parent_hash != common.to_string() {
                    new_branch_tips.push(parent_hash);
                }
            }
        }
        *branch_tips = new_branch_tips;
        Ok(())
    }

    pub fn open_config(&self) -> Result<Config, CommandError> {
        Config::open(&self.git_path)
    }

    pub fn update_remote(&self, url: String) -> Result<(), CommandError> {
        let mut config = self.open_config()?;
        let fetch = format!("+refs/heads/*:refs/remotes/origin/*");
        config.insert("remote \"origin\"", "url", &url);
        config.insert("remote \"origin\"", "fetch", &fetch);
        config.save()?;
        Ok(())
    }

    /// Ejecuta el comando fetch.\
    /// Obtiene los objetos del servidor y guarda nuevos objetos en la base de datos.\
    /// Actualiza la referencia `FETCH_HEAD` con el hash del último commit de cada rama.
    pub fn fetch(&mut self) -> Result<(), CommandError> {
        self.log("Fetching updates");
        let (address, repository_path, repository_url) = self.get_remote_info()?;
        self.log(&format!(
            "Address: {}, repository_path: {}, repository_url: {}",
            address, repository_path, repository_url
        ));
        let mut server = GitServer::connect_to(&address)?;

        let _remote_branches =
            self.update_remote_branches(&mut server, &repository_path, &repository_url)?;
        let remote_reference = format!("{}:{}", address, repository_path);
        self.fetch_and_save_objects(&mut server, &remote_reference)?;
        Ok(())
    }

    /// Abre el archivo config de la base de datos\
    /// obtiene el address, repository_path y repository_url del remote origin\
    fn get_remote_info(&mut self) -> Result<(String, String, String), CommandError> {
        let config = self.open_config()?;
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
        Ok((
            address.to_owned(),
            repository_path.to_owned(),
            repository_url.to_owned(),
        ))
    }

    /// Actualiza la base de datos con los nuevos objetos recibidos del servidor.
    fn fetch_and_save_objects(
        &mut self,
        server: &mut GitServer,
        remote_reference: &str,
    ) -> Result<(), CommandError> {
        let remote_branches = self.remote_branches()?;
        let wants = remote_branches.clone().into_values().collect();
        let haves = self
            .get_fetch_head_commits()?
            .into_values()
            .filter_map(|(commit_hash, _should_merge)| Some(commit_hash))
            .collect();
        self.log(&format!("Wants {:#?}", wants));
        self.log(&format!("haves {:#?}", haves));
        let objects_decompressed_data = server.fetch_objects(wants, haves, &mut self.logger)?;
        self.save_objects_from_packfile(objects_decompressed_data)?;

        self.update_fetch_head(remote_branches, remote_reference)?;
        Ok(())
    }

    pub fn save_objects_from_packfile(
        &mut self,
        objects_decompressed_data: Vec<(PackfileObjectType, usize, Vec<u8>)>,
    ) -> Result<HashMap<String, (PackfileObjectType, usize, Vec<u8>)>, CommandError> {
        let mut objects = HashMap::<String, (PackfileObjectType, usize, Vec<u8>)>::new();
        for (obj_type, len, content) in objects_decompressed_data {
            self.log(&format!(
                "Saving object of type {} and len {}, with data {:?}",
                obj_type,
                len,
                String::from_utf8_lossy(&content)
            ));
            let mut git_object: GitObject =
                Box::new(ProtoObject::new(content.clone(), len, obj_type.to_string()));
            let hash = self.db()?.write(&mut git_object, false, &mut self.logger)?;
            objects.insert(hash, (obj_type, len, content));
        }
        Ok(objects)
    }

    pub fn log(&mut self, content: &str) {
        self.logger.log(content);
    }

    pub fn logger(&mut self) -> &mut Logger {
        &mut self.logger
    }

    pub fn pull(&mut self) -> Result<(), CommandError> {
        self.fetch()?;
        self.merge(&Vec::new())?;
        Ok(())
    }

    pub fn push(&mut self, local_branches: Vec<(String, String)>) -> Result<(), CommandError> {
        self.log("Push updates");
        let (address, repository_path, repository_url) = self.get_remote_info()?;
        self.log(&format!(
            "Address: {}, repository_path: {}, repository_url: {}",
            address, repository_path, repository_url
        ));

        let mut server = GitServer::connect_to(&address)?;
        let refs_hash = self.receive_pack(&mut server, &repository_path, &repository_url)?; // ref_hash: HashMap<branch, hash>

        // verificamos que todas las branches locales esten actualizadas
        let (_, _, repository_url) = self.get_remote_info()?;

        let (hash_branch_status, commits_map) =
            get_analysis(local_branches, self.db()?, refs_hash, &mut self.logger).map_err(
                |error| match error {
                    CommandError::PushBranchBehind(local_branch) => {
                        CommandError::PushBranchBehindVerbose(repository_url, local_branch)
                    }
                    _ => CommandError::PushBranchesError,
                },
            )?;

        if hash_branch_status.is_empty() {
            self.log("Everything up-to-date");
            self.output
                .write_all(b"Everything up-to-date")
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
            return Ok(());
        }

        self.log(&format!("hash_branch_status: {:?}", hash_branch_status));

        server.negociate_recieve_pack(hash_branch_status)?;

        self.log("Sending packfile");
        let pack_file: Vec<u8> = make_packfile(commits_map)?;
        self.log(&format!(
            "pack_file: {:?}",
            String::from_utf8_lossy(&pack_file)
        ));

        server.write_to_socket(&pack_file)?;
        self.log("sent! Reading response");
        println!("sent! Reading response");

        let response = server.get_response()?;
        // let response = server.just_read()?;
        self.log(&format!("response: {:?}", response));

        Ok(())
    }

    /// Actualiza todas las branches de la carpeta remotes con los hashes de los commits
    /// obtenidos del servidor.
    fn receive_pack(
        &mut self,
        server: &mut GitServer,
        repository_path: &str,
        repository_url: &str,
    ) -> Result<HashMap<String, String>, CommandError> {
        self.log("git receive-pack");
        let hash_refs = server
            .explore_repository_receive_pack(&("/".to_owned() + repository_path), repository_url)?;
        self.log(&format!("hash_refs: {:?}", hash_refs));
        Ok(hash_refs)
    }

    pub fn merge(&mut self, commits: &Vec<String>) -> Result<(), CommandError> {
        self.log(&format!("Merge args: {:?}", commits));
        let mut head_name = "HEAD".to_string();
        let mut destin_name = "origin".to_string();
        if commits.len() > 1 {
            return Err(CommandError::MergeMultipleCommits);
        }
        let (mut destin_commit, is_remote) = match commits.first() {
            Some(commit) if commit != "FETCH_HEAD" => (commit.to_owned(), false),
            _ => (self.get_fetch_head_branch_commit_hash()?, true),
        };

        match self.get_last_commit_hash()? {
            Some(last_commit) => {
                let mut destin_branch_name = "".to_string();
                let mut head_branch_name = "".to_string();
                for (branch_name, branch_hash) in self.remote_branches()? {
                    self.log(&format!(
                        "Remote branch_name: {}, branch_hash: {}",
                        branch_name, branch_hash
                    ));
                    if branch_name == destin_commit {
                        destin_commit = branch_hash.clone();
                    }
                    if branch_hash == destin_commit {
                        destin_branch_name = branch_name;
                        //return Ok(());
                    }
                }
                for (branch_name, branch_hash) in self.local_branches()? {
                    self.log(&format!(
                        "Local branch_name: {}, branch_hash: {}",
                        branch_name, branch_hash
                    ));
                    if branch_name == destin_commit {
                        destin_commit = branch_hash.clone();
                    }
                    if branch_hash == destin_commit {
                        destin_branch_name = branch_name.clone();
                        //return Ok(());
                    }
                    if branch_hash == last_commit {
                        head_branch_name = branch_name;
                        //return Ok(());
                    }
                }
                self.log(&format!(
                    "Local branch_name: {}, branch_hash: {}, Remote branch_name: {}, branch hash: {}",
                    head_branch_name, last_commit, destin_branch_name, destin_commit
                ));
                if destin_branch_name != head_branch_name
                    || destin_branch_name.is_empty()
                    || destin_branch_name.is_empty()
                {
                    if destin_branch_name.is_empty() {
                        destin_name = destin_commit.to_owned();
                    } else {
                        if is_remote {
                            destin_name = format!("{}/origin", destin_branch_name);
                        } else {
                            destin_name = destin_branch_name;
                        };
                    }
                    if head_branch_name.is_empty() {
                        head_name = last_commit.to_owned();
                    } else {
                        if is_remote {
                            head_name = format!("{}/HEAD", head_branch_name);
                        } else {
                            head_name = head_branch_name;
                        };
                    }
                }
                self.log(&format!("Merging two commits ...",));
                self.merge_two_commits(&last_commit, &destin_commit, &head_name, &destin_name)
            }
            None => self.merge_fast_forward(&destin_commit),
        }
    }

    /// Obtiene la ruta de la rama actual.\
    /// formato: `refs/heads/branch_name`
    pub fn get_head_branch_path(&mut self) -> Result<String, CommandError> {
        let mut branch = String::new();
        let path =
            join_paths!(self.git_path, "HEAD").ok_or(CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ))?;
        let Ok(mut head) = File::open(&path) else {
            return Err(CommandError::NotGitRepository);
        };

        if head.read_to_string(&mut branch).is_err() {
            return Err(CommandError::FileReadError(path.to_string()));
        }

        let branch = branch.trim();
        let Some(branch) = branch.split(" ").last() else {
            return Err(CommandError::HeadError);
        };
        Ok(branch.to_string())
    }

    /// Obtiene el nombre de la rama actual.
    pub fn get_current_branch_name(&mut self) -> Result<String, CommandError> {
        let branch = self.get_head_branch_path()?;
        let branch_name: Vec<&str> = branch.split_terminator("/").collect();
        Ok(branch_name[branch_name.len() - 1].to_string())
    }

    /// Obtiene el hash del Commit HEAD.
    pub fn get_last_commit_hash(&mut self) -> Result<Option<String>, CommandError> {
        let mut parent = String::new();
        let branch = self.get_head_branch_path()?;
        let branch_path =
            join_paths!(&self.git_path, branch).ok_or(CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ))?;
        let Ok(mut branch_file) = File::open(branch_path.clone()) else {
            return Ok(None);
        };

        if branch_file.read_to_string(&mut parent).is_err() {
            return Err(CommandError::FileReadError(branch_path.to_string()));
        }

        let parent = parent.trim();
        Ok(Some(parent.to_string()))
    }

    pub fn local_branches(&mut self) -> Result<HashMap<String, String>, CommandError> {
        let mut branches = HashMap::<String, String>::new();
        let branches_path = join_paths!(&self.git_path, "refs/heads/").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;
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
            branches.insert(file_name.trim().to_string(), sha1.trim().to_string());
        }
        Ok(branches)
    }

    /// Abre la base de datos en la carpeta . git/refs/remotes/origin y obtiene los hashes de los
    /// commits de las branches remotos.
    /// Devuelve un HashMap con el formato: `{nombre_branch: hash_commit}`.
    fn remote_branches(&mut self) -> Result<HashMap<String, String>, CommandError> {
        let mut branches = HashMap::<String, String>::new();
        let branches_path = join_paths!(&self.git_path, "refs/remotes/origin/").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;
        let Ok(paths) = fs::read_dir(branches_path) else {
            return Ok(branches);
        };
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

    /// Actualiza todas las branches de la carpeta remotes con los hashes de los commits
    /// obtenidos del servidor.
    fn update_remote_branches(
        &mut self,
        server: &mut GitServer,
        repository_path: &str,
        repository_url: &str,
    ) -> Result<HashMap<String, String>, CommandError> {
        self.log("Updating remote branches");
        let (_head_branch, branch_remote_refs) = server
            .explore_repository_upload_pack(&("/".to_owned() + repository_path), repository_url)?;
        self.log(&format!("branch_remote_refs: {:?}", branch_remote_refs));

        let mut remote_branches = HashMap::<String, String>::new();

        for (hash, mut branch_name) in branch_remote_refs {
            branch_name.replace_range(0..11, "");
            self.update_ref(&hash, &branch_name)?;
            remote_branches.insert(branch_name, hash);
        }

        Ok(remote_branches)
    }

    /// Devuelve al vector de branches_with_their_commits todos los nombres de las ramas y el hash del commit al que apuntan
    pub fn push_all_local_branch_hashes(&mut self) -> Result<Vec<(String, String)>, CommandError> {
        let mut branches_with_their_commits: Vec<(String, String)> = Vec::new();
        let branches_hashes = self.local_branches()?;
        for branch_hash in branches_hashes {
            let branch_hash = (branch_hash.0, branch_hash.1.to_string());
            branches_with_their_commits.push(branch_hash);
        }
        Ok(branches_with_their_commits)
    }

    /// Devuelve al vector de branches_with_their_commits todos los nombres de las ramas y el hash del commit al que apuntan
    pub fn push_all_remote_branch_hashes(&mut self) -> Result<Vec<(String, String)>, CommandError> {
        let mut branches_with_their_commits: Vec<(String, String)> = Vec::new();
        let branches_hashes = self.remote_branches()?;
        for branch_hash in branches_hashes {
            let branch_hash = (branch_hash.0, branch_hash.1.to_string());
            branches_with_their_commits.push(branch_hash);
        }
        Ok(branches_with_their_commits)
    }

    /// Devuelve al vector de branches_with_their_commits todos los nombres de las ramas y el hash del commit al que apuntan
    pub fn push_all_branch_hashes(&mut self) -> Result<Vec<(String, String)>, CommandError> {
        let mut branches_with_their_commits: Vec<(String, String)> = Vec::new();
        let branches_local_hashes = self.local_branches()?;

        for branch_local_hash in branches_local_hashes {
            let branch_local_hash = (branch_local_hash.0, branch_local_hash.1.to_string());
            branches_with_their_commits.push(branch_local_hash);
        }
        let branches_remotes_hashes = match self.remote_branches() {
            Ok(remote_branches) => remote_branches,
            Err(_) => {
                return Ok(branches_with_their_commits);
            }
        };
        for branch_remote_hash in branches_remotes_hashes {
            let branch_name_remote = format!("origin/{}", branch_remote_hash.0);
            let branch_remote_hash = (branch_name_remote, branch_remote_hash.1.to_string());
            branches_with_their_commits.push(branch_remote_hash);
        }
        Ok(branches_with_their_commits)
    }

    /// Actualiza la referencia de la branch con el hash del commit obtenido del servidor.
    fn update_ref(&mut self, sha1: &str, ref_name: &str) -> Result<(), CommandError> {
        let dir_path = join_paths!(&self.git_path, "refs/remotes/origin/").ok_or(
            CommandError::DirectoryCreationError("Error creando directorio de refs".to_string()),
        )?;
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

    /// Actualiza el archivo FETCH_HEAD con los hashes de los commits obtenidos del servidor.
    fn update_fetch_head(
        &mut self,
        remote_branches: HashMap<String, String>,
        remote_reference: &str,
    ) -> Result<(), CommandError> {
        self.log("Updating FETCH_HEAD");
        let fetch_head_path = join_paths!(&self.git_path, "FETCH_HEAD").ok_or(
            CommandError::DirectoryCreationError("Error actualizando FETCH_HEAD".to_string()),
        )?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&fetch_head_path)
            .map_err(|error| {
                CommandError::FileWriteError(format!(
                    "Error abriendo FETCH_HEAD en {}: {}",
                    fetch_head_path,
                    &error.to_string()
                ))
            })?;

        let head_branch_name = self.get_head_branch_name()?;
        let head_branch_hash = remote_branches
            .get(&head_branch_name)
            .ok_or(CommandError::NoHeadCommit(head_branch_name.to_string()))?;

        let line = format!(
            "{}\t\tbranch '{}' of {}",
            head_branch_hash, head_branch_name, remote_reference
        );
        file.write_all(line.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(
                "Error guardando FETCH_HEAD:".to_string() + &error.to_string(),
            )
        })?;
        for (branch_name, hash) in remote_branches {
            if branch_name == head_branch_name {
                continue;
            }
            let line = format!(
                "{}\tnot-for-merge\tbranch '{}' of {}",
                hash, branch_name, &remote_reference
            );
            file.write_all(line.as_bytes()).map_err(|error| {
                CommandError::FileWriteError(
                    "Error guardando FETCH_HEAD:".to_string() + &error.to_string(),
                )
            })?;
        }
        Ok(())
    }

    pub fn get_head_branch_name(&mut self) -> Result<String, CommandError> {
        let head_branch_path = self.get_head_branch_path()?;
        let head_branch_name =
            head_branch_path
                .split("/")
                .last()
                .ok_or(CommandError::FileWriteError(
                    "No se pudo obtener el nombre de la rama".to_string(),
                ))?;
        Ok(head_branch_name.to_owned())
    }

    /// Devuelve el hash del commit que apunta la rama que se hizo fetch dentro de `FETCH_HEAD` (commit del remoto).
    fn get_fetch_head_branch_commit_hash(&self) -> Result<String, CommandError> {
        let fetch_head_path =
            join_paths!(&self.git_path, "FETCH_HEAD").ok_or(CommandError::JoiningPaths)?;

        let Ok(mut fetch_head_file) = fs::File::open(fetch_head_path) else {
            return Err(CommandError::FileReadError(
                "Error leyendo FETCH_HEAD".to_string(),
            ));
        };
        let mut fetch_head_content = String::new();
        fetch_head_file
            .read_to_string(&mut fetch_head_content)
            .map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo FETCH_HEAD: {:?}",
                    error.to_string()
                ))
            })?;
        let mut lines = fetch_head_content.lines();
        let first_line = lines.next().ok_or(CommandError::FileReadError(
            "Error leyendo FETCH_HEAD".to_string(),
        ))?;
        let branch_data: Vec<&str> = first_line.split('\t').collect();
        let branch_hash = branch_data[0];
        let _should_merge = branch_data[1];
        let branch_info = branch_data[2].to_string();
        let (_branch_name, _remote_info) =
            branch_info
                .split_once("\' of")
                .ok_or(CommandError::FileReadError(
                    "Error leyendo FETCH_HEAD".to_string(),
                ))?;
        Ok(branch_hash.to_owned())
    }

    /// Devuelve el hash del commit que apunta la rama que se hizo fetch
    fn get_fetch_head_commits(&self) -> Result<HashMap<String, (String, bool)>, CommandError> {
        let fetch_head_path =
            join_paths!(&self.git_path, "FETCH_HEAD").ok_or(CommandError::JoiningPaths)?;

        let mut branches = HashMap::<String, (String, bool)>::new();
        let Ok(mut fetch_head_file) = fs::File::open(fetch_head_path) else {
            return Ok(branches);
        };
        let mut fetch_head_content = String::new();
        fetch_head_file
            .read_to_string(&mut fetch_head_content)
            .map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo FETCH_HEAD: {:?}",
                    error.to_string()
                ))
            })?;
        let mut lines = fetch_head_content.lines();
        while let Some(line) = lines.next() {
            let branch_data: Vec<&str> = line.split('\t').collect();
            let commit_hash = branch_data[0];
            let should_merge_str = branch_data[1];
            let branch_info = branch_data[2].to_string();
            let (branch_name, _remote_info) =
                branch_info
                    .split_once("\' of")
                    .ok_or(CommandError::FileReadError(
                        "Error leyendo FETCH_HEAD".to_string(),
                    ))?;
            let should_merge = !(should_merge_str == "not-for-merge");
            branches.insert(
                branch_name.to_owned(),
                (commit_hash.to_owned(), should_merge),
            );
        }

        Ok(branches)
    }

    /// Es el merge feliz, donde no hay conflictos. Se reemplaza el working tree por el del commit
    /// del remoto.
    fn merge_fast_forward(&mut self, destin_commit: &str) -> Result<(), CommandError> {
        self.log("Merge fast forward");
        self.set_head_branch_commit_to(destin_commit)?;
        let tree = self
            .get_last_commit_tree()?
            .ok_or(CommandError::FileWriteError(
                "Error en restore:".to_string(),
            ))?;

        self.log("updated stagin area for merge fast foward");
        self.restore(tree)?;
        Ok(())
    }

    /// Guarda en el archivo de la rama actual el hash del commit que se quiere hacer merge.
    fn set_head_branch_commit_to(&mut self, commits: &str) -> Result<(), CommandError> {
        let branch = self.get_head_branch_path()?;

        self.write_to_file(&branch, commits)?;
        Ok(())
    }

    fn restore(&mut self, mut source_tree: Tree) -> Result<(), CommandError> {
        self.log("Restoring files");
        source_tree.restore(&self.working_dir_path, &mut self.logger)?;
        let mut staging_area = self.staging_area()?;
        staging_area.update_to_tree(&source_tree)?;
        staging_area.save()?;
        Ok(())
    }

    fn restore_merge_conflict(&mut self, mut source_tree: Tree) -> Result<(), CommandError> {
        self.log("Restoring files");
        source_tree.restore(&self.working_dir_path, &mut self.logger)?;
        Ok(())
    }

    pub fn db(&self) -> Result<ObjectsDatabase, CommandError> {
        ObjectsDatabase::new(&self.git_path)
    }

    /// Devuelve true si Git no reconoce el path pasado.
    fn is_untracked(
        &mut self,
        path: &str,
        staging_area: &StagingArea,
    ) -> Result<bool, CommandError> {
        /* let mut blob = Blob::new_from_path(path.to_string())?;
        let hash = &blob.get_hash_string()?; */
        /* let (is_in_last_commit, name) = self.is_in_last_commit_from_hash(hash.to_owned())?;
        if staging_area.contains_key(path) || (is_in_last_commit && name == get_name(&path)?) {
            return Ok(false);
        } */
        let last_commit = &self.get_last_commit_tree()?;
        let is_in_last_commit = self.is_in_last_commit_from_path(path, last_commit)
            || staging_area.has_file_from_path(path);
        self.log(&format!(
            "{} is in last commit? {}",
            path, is_in_last_commit
        ));
        if !is_in_last_commit {
            return Ok(true);
        }
        Ok(false)
    }

    /// Guarda en el stagin area el estado actual del working tree, sin tener en cuenta los archivos
    /// nuevos.
    fn save_entries(
        &mut self,
        path_name: &str,
        staging_area: &mut StagingArea,
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
            self.log(&format!("Entry: {}", entry_name));

            if entry_name.contains(".git") {
                continue;
            }
            if entry_path.is_dir() {
                self.save_entries(&entry_name, staging_area, files)?;
            } else {
                let blob = Blob::new_from_path(entry_name.to_string())?;
                let path = &entry_name[2..];
                if !self.is_untracked(path, &staging_area)? {
                    let mut git_object: GitObject = Box::new(blob);
                    self.log(&format!("Adding {} to staging area", path));
                    let hex_str = self.db()?.write(&mut git_object, false, &mut self.logger)?;
                    staging_area.add(path, &hex_str);
                }
            }
        }
        Ok(())
    }

    pub fn get_last_commit_tree(&mut self) -> Result<Option<Tree>, CommandError> {
        self.log("get_last_commit_tree");

        let Some(last_commit) = self.get_last_commit_hash()? else {
            return Ok(None);
        };
        self.log(&format!(
            "Last commit found in get_last_commit_tree : {}",
            last_commit
        ));

        let mut commit_box = self.db()?.read_object(&last_commit, &mut self.logger)?;
        self.log("Last commit read");
        if let Some(commit) = commit_box.as_mut_commit() {
            self.log(&format!(
                "Last commit content : {}",
                String::from_utf8_lossy(&commit.content(None)?)
            ));
            let option_tree = commit.get_tree();

            let Some(tree) = option_tree else {
                return Ok(None);
            };

            self.log(&format!(
                "tree content : {}",
                String::from_utf8_lossy(&(tree.to_owned().content(None)?))
            ));
            return Ok(Some(tree.to_owned()));
        }
        self.log("not found commit");
        Ok(None)
    }

    fn merge_two_commits(
        &mut self,
        head_commit: &str,
        destin_commit: &str,
        head_name: &str,
        destin_name: &str,
    ) -> Result<(), CommandError> {
        self.log("Running merge_commits");

        let (mut common, mut commit_head, mut commit_destin) =
            self.get_common_ansestor(&destin_commit, head_commit)?;

        let hash = common.get_hash_string()?;
        self.log(&format!("Common hash: {}", hash));
        if common.get_hash()? == commit_head.get_hash()? {
            return self.merge_fast_forward(&destin_commit);
        }

        self.log("True merge");
        self.true_merge(
            &mut common,
            &mut commit_head,
            &mut commit_destin,
            &head_name,
            &destin_name,
        )
    }

    fn get_common_ansestor(
        &mut self,
        commit_destin_str: &str,
        commit_head_str: &str,
    ) -> Result<(CommitObject, CommitObject, CommitObject), CommandError> {
        self.log("Get common ansestor inicio");
        self.log(&format!(
            "commit_destin_str: {}, commit_head_str: {}",
            commit_destin_str, commit_head_str
        ));

        let commit_head = self
            .db()?
            .read_object(&commit_head_str, &mut self.logger)?
            .as_mut_commit()
            .ok_or(CommandError::FailedToFindCommonAncestor)?
            .to_owned();
        let commit_destin = self
            .db()?
            .read_object(commit_destin_str, &mut self.logger)?
            .as_mut_commit()
            .ok_or(CommandError::FailedToFindCommonAncestor)?
            .to_owned();
        self.log("Found objects");
        let mut head_branch_commits: HashMap<String, CommitObject> = HashMap::new();
        head_branch_commits.insert(commit_head_str.to_string(), commit_head.clone());

        let mut destin_branch_commits: HashMap<String, CommitObject> = HashMap::new();
        destin_branch_commits.insert(commit_destin_str.to_string(), commit_destin.clone());

        let mut head_branch_tips: Vec<CommitObject> = [commit_head.clone()].to_vec();
        let mut destin_branch_tips: Vec<CommitObject> = [commit_destin.clone()].to_vec();

        loop {
            if head_branch_tips.is_empty() && destin_branch_tips.is_empty() {
                break;
            }
            for tip_commit in head_branch_tips.iter_mut() {
                let hash_string = tip_commit.get_hash_string()?;
                self.log(&format!("head_hash_string: {}", hash_string));

                self.log(&format!(
                    "destin keys: {:?}",
                    destin_branch_commits.keys().collect::<Vec<&String>>()
                ));
                if destin_branch_commits.contains_key(&hash_string) {
                    return Ok((tip_commit.to_owned(), commit_head, commit_destin));
                }
            }

            for tip_commit in destin_branch_tips.iter_mut() {
                let hash_string = tip_commit.get_hash_string()?;
                self.log(&format!("destin_hash_string: {}", hash_string));
                let get_hash_string = tip_commit.get_hash_string()?;
                self.log(&format!(
                    "head keys: {:?}",
                    head_branch_commits.keys().collect::<Vec<&String>>()
                ));
                if head_branch_commits.contains_key(&get_hash_string) {
                    return Ok((tip_commit.to_owned(), commit_head, commit_destin));
                }
            }
            self.read_row(&mut head_branch_tips, &mut head_branch_commits)?;
            self.read_row(&mut destin_branch_tips, &mut destin_branch_commits)?;
        }
        Err(CommandError::FailedToFindCommonAncestor)
    }

    fn read_row(
        &mut self,
        branch_tips: &mut Vec<CommitObject>,
        branch_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError> {
        let mut new_branch_tips = Vec::<CommitObject>::new();
        for tip in branch_tips.iter() {
            let parents_hash = tip.get_parents();
            for parent_hash in parents_hash {
                let parent = self
                    .db()?
                    .read_object(&parent_hash, &mut self.logger)?
                    .as_mut_commit()
                    .ok_or(CommandError::FailedToFindCommonAncestor)?
                    .to_owned();

                if branch_commits.insert(parent_hash, parent.clone()).is_none() {
                    new_branch_tips.push(parent);
                }
            }
        }
        *branch_tips = new_branch_tips;
        Ok(())
    }

    /// Obtiene el hash del commit al que apunta la rama que se le pasa por parámetro
    pub fn get_last_commit_hash_branch(
        &self,
        refs_branch_name: &String,
    ) -> Result<String, CommandError> {
        let path_to_branch = join_paths!(self.git_path, "refs/heads", refs_branch_name)
            .ok_or(CommandError::FileOpenError(refs_branch_name.clone()))?;

        let mut file = File::open(&path_to_branch).map_err(|_| {
            CommandError::FileNotFound(format!("No se pudo abrir {path_to_branch} en log"))
        })?;

        let mut commit_hash = String::new();
        file.read_to_string(&mut commit_hash).map_err(|_| {
            CommandError::FileReadError(format!("No se pudo leer {path_to_branch} en log"))
        })?;

        Ok(commit_hash[..commit_hash.len()].to_string())
    }

    pub fn get_last_commit_hash_branch_local_remote(
        &self,
        refs_branch_name: &String,
    ) -> Result<Vec<(String, String)>, CommandError> {
        let mut branches_with_their_commits: Vec<(String, String)> = Vec::new();
        let path_to_branch_local = join_paths!(self.git_path, "refs/heads", refs_branch_name)
            .ok_or(CommandError::FileOpenError(refs_branch_name.clone()))?;

        let mut file = File::open(&path_to_branch_local).map_err(|_| {
            CommandError::FileNotFound(format!("No se pudo abrir {path_to_branch_local} en log"))
        })?;

        let mut commit_hash_local = String::new();
        file.read_to_string(&mut commit_hash_local).map_err(|_| {
            CommandError::FileReadError(format!("No se pudo leer {path_to_branch_local} en log"))
        })?;

        branches_with_their_commits.push((refs_branch_name.to_owned(), commit_hash_local));

        let path_to_branch_remote =
            join_paths!(self.git_path, "refs/remotes/origin", refs_branch_name)
                .ok_or(CommandError::FileOpenError(refs_branch_name.clone()))?;

        let mut file = match File::open(&path_to_branch_remote) {
            Ok(file) => file,
            Err(_) => {
                return Ok(branches_with_their_commits);
            }
        };

        let mut commit_hash_remote = String::new();
        file.read_to_string(&mut commit_hash_remote).map_err(|_| {
            CommandError::FileReadError(format!("No se pudo leer {path_to_branch_remote} en log"))
        })?;

        let branch_name_remote = format!("origin/{}", refs_branch_name);

        branches_with_their_commits.push((branch_name_remote.to_owned(), commit_hash_remote));
        Ok(branches_with_their_commits)
    }

    fn true_merge(
        &mut self,
        common: &mut CommitObject,
        head: &mut CommitObject,
        destin: &mut CommitObject,
        head_name: &str,
        destin_name: &str,
    ) -> Result<(), CommandError> {
        let mut common_tree = common.get_tree_some_or_err()?.to_owned();
        let mut head_tree = head.get_tree_some_or_err()?.to_owned();
        let mut destin_tree = destin.get_tree_some_or_err()?.to_owned();

        let mut staging_area = self.staging_area()?;
        // staging_area.update_to_conflictings(merged_files.to_owned(), unmerged_files.to_owned());
        staging_area.clear();
        let merged_tree = merge_trees(
            &mut head_tree,
            &mut destin_tree,
            &mut common_tree,
            head_name,
            destin_name,
            &self.working_dir_path,
            &mut staging_area,
            &mut self.logger,
        )?;
        staging_area.save()?;

        let message = format!("Merge branch '{}' into {}", destin_name, head_name);
        if staging_area.has_conflicts() {
            self.log(&format!(
                "Conflicts {:?}",
                staging_area.get_unmerged_files()
            ));
            let mut boxed_tree: GitObject = Box::new(merged_tree.clone());
            let merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;

            self.write_to_file("MERGE_MSG", &message)?;
            self.write_to_file("AUTO_MERGE", &merge_tree_hash_str)?;
            self.write_to_file("MERGE_HEAD", &destin.get_hash_string()?)?;

            self.restore_merge_conflict(merged_tree)?;
            Ok(())
        } else {
            let mut boxed_tree: GitObject = Box::new(merged_tree.clone());
            let _merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;
            let merge_commit = self.create_new_commit(
                message,
                [head.get_hash_string()?, destin.get_hash_string()?].to_vec(),
                merged_tree.clone(),
            )?;

            let mut boxed_commit: GitObject = Box::new(merge_commit.clone());
            let merge_commit_hash_str =
                self.db()?
                    .write(&mut boxed_commit, false, &mut self.logger)?;
            self.set_head_branch_commit_to(&merge_commit_hash_str)?;
            self.restore(merged_tree)?;
            Ok(())
        }
    }

    /// Tries to continue from failed merged
    pub fn merge_continue(&mut self) -> Result<(), CommandError> {
        let (message, merge_tree_hash_str, destin) = self.get_failed_merge_info()?;
        let staging_area = self.staging_area()?;
        if staging_area.has_conflicts() {
            return Err(CommandError::UnmergedFiles);
        }
        let get_last_commit_hash = self
            .get_last_commit_hash()?
            .ok_or(CommandError::FailedToResumeMerge)?;
        let parents = [get_last_commit_hash, destin].to_vec();
        let merge_tree = self
            .db()?
            .read_object(&merge_tree_hash_str, &mut self.logger)?
            .as_mut_tree()
            .ok_or(CommandError::FailedToResumeMerge)?
            .to_owned();
        let merge_commit = self.create_new_commit(message, parents, merge_tree.clone())?;
        let mut boxed_commit: GitObject = Box::new(merge_commit.clone());
        let merge_commit_hash_str = self
            .db()?
            .write(&mut boxed_commit, false, &mut self.logger)?;
        self.set_head_branch_commit_to(&merge_commit_hash_str)?;
        self.restore(merge_tree)?;
        self.delete_file("MERGE_MSG")?;
        self.delete_file("AUTO_MERGE")?;
        self.delete_file("MERGE_HEAD")?;
        Ok(())
    }

    fn staging_area(&mut self) -> Result<StagingArea, CommandError> {
        Ok(StagingArea::open(&self.git_path)?)
    }

    fn get_failed_merge_info(&mut self) -> Result<(String, String, String), CommandError> {
        let (Ok(message), Ok(merge_tree_hash_str), Ok(destin)) = (
            self.read_file("MERGE_MSG"),
            self.read_file("AUTO_MERGE"),
            self.read_file("MERGE_HEAD"),
        ) else {
            return Err(CommandError::NoMergeFound);
        };

        Ok((message, merge_tree_hash_str, destin))
    }

    /// Dado un path, crea el archivo correspondiente y escribe el contenido pasado.
    fn write_to_file(&self, relative_path: &str, content: &str) -> Result<(), CommandError> {
        let path_f = join_paths!(self.git_path, relative_path).ok_or(
            CommandError::FileWriteError("Error guardando FETCH_HEAD:".to_string()),
        )?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path_f)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        file.write_all(content.as_bytes())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    fn read_file(&self, relative_path: &str) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, relative_path).ok_or(
            CommandError::FileWriteError("Error guardando FETCH_HEAD:".to_string()),
        )?;
        let mut file = fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(content)
    }

    fn delete_file(&self, relative_path: &str) -> Result<(), CommandError> {
        let path = join_paths!(self.git_path, relative_path).ok_or(CommandError::JoiningPaths)?;
        fs::remove_file(path).map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error borrando archivo {}: {}",
                relative_path,
                error.to_string()
            ))
        })?;
        Ok(())
    }

    pub fn get_log(
        &mut self,
        all: bool,
    ) -> Result<Vec<(CommitObject, Option<String>)>, CommandError> {
        let mut branches_with_their_last_hash: Vec<(String, String)> = Vec::new();
        let mut commits_map: HashMap<String, (CommitObject, Option<String>)> = HashMap::new();

        if all {
            branches_with_their_last_hash = self.push_all_branch_hashes()?;
        } else {
            let current_branch = get_head_ref()?;
            let current_branch_name = current_branch[11..].to_string();
            let hash_commit = self.get_last_commit_hash_branch(&current_branch_name)?;
            branches_with_their_last_hash.push((current_branch, hash_commit));
        }
        for branch_with_commit in branches_with_their_last_hash {
            rebuild_commits_tree(
                &self.db()?,
                &branch_with_commit.1,
                &mut commits_map,
                Some(branch_with_commit.0),
                all,
                &HashSet::<String>::new(),
                false,
                &mut self.logger,
            )?;
        }

        let mut commits = commits_map.drain().map(|(_, v)| v).collect();
        sort_commits_descending_date(&mut commits);
        Ok(commits)
    }

    pub fn get_stage_and_unstage_changes(
        &mut self,
    ) -> Result<(HashSet<String>, HashSet<String>), CommandError> {
        let last_commit_tree = match self.get_last_commit_tree() {
            Ok(tree) => tree,
            Err(err) => {
                eprintln!("Error al obtener el último commit: {}", err);
                return Err(CommandError::FileWriteError(err.to_string()));
            }
        };
        let index = self.staging_area()?;

        let changes_controller = ChangesController::new(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            &mut self.logger,
            last_commit_tree,
            &index,
        )
        .unwrap();

        let changes_to_be_commited_vec =
            sort_hashmap_and_filter_unmodified(changes_controller.get_changes_to_be_commited());
        let changes_to_be_commited: HashSet<String> = changes_to_be_commited_vec
            .into_iter()
            .map(|(s, _)| s)
            .collect();

        let changes_not_staged_vec =
            sort_hashmap_and_filter_unmodified(changes_controller.get_changes_not_staged());
        let mut changes_not_staged: HashSet<String> =
            changes_not_staged_vec.into_iter().map(|(s, _)| s).collect();

        let untracked_files_vec = changes_controller.get_untracked_files_bis();

        println!("untracked_files_vec: {:?}", untracked_files_vec);

        changes_not_staged.extend(untracked_files_vec.iter().cloned());

        return Ok((changes_to_be_commited, changes_not_staged));
    }
    // ----- Branch -----

    fn change_current_branch_name(&mut self, new_name: &str) -> Result<(), CommandError> {
        let path = join_paths!(self.git_path, "HEAD").ok_or(CommandError::FileCreationError(
            " No se pudo obtener .git/HEAD".to_string(),
        ))?;
        let mut file =
            File::create(path).map_err(|error| CommandError::FileOpenError(error.to_string()))?;

        file.write_all(format!("ref: refs/heads/{}", new_name).as_bytes())
            .map_err(|error| {
                CommandError::FileWriteError(
                    "Error guardando ref de HEAD:".to_string() + &error.to_string(),
                )
            })?;
        self.log("HEAD renamed");
        Ok(())
    }

    pub fn rename_branch(&mut self, branches: &Vec<String>) -> Result<(), CommandError> {
        let head = self.get_current_branch_name()?;
        if branches.len() == 1 {
            let new_name = branches[0].clone();
            self.change_branch_name(&head, &new_name)?;
            self.change_current_branch_name(&new_name)?;
            self.log(&format!("Branch '{head}' has been renamed to '{new_name}'"))
        } else {
            self.change_branch_name(&branches[0], &branches[1])?;
            self.log(&format!(
                "Branch '{}' has been renamed to '{}'",
                branches[0], branches[1]
            ));
            if branches[0] == head {
                self.change_current_branch_name(&branches[1])?;
            }
        }
        Ok(())
    }

    pub fn create_branch(&mut self, new_branch_info: &Vec<String>) -> Result<(), CommandError> {
        let commit_hash: String;
        let mut new_is_remote = false;
        if new_branch_info.len() == 2 {
            if let Some(hash) = self.try_read_commit_local_branch(&new_branch_info[1])? {
                commit_hash = hash;
            } else if let Some(hash) = self.try_read_commit(&new_branch_info[1])? {
                commit_hash = hash;
            } else if let Some(hash) = self.try_read_commit_remote_branch(&new_branch_info[1])? {
                commit_hash = hash;
                new_is_remote = true;
            } else {
                return Err(CommandError::InvalidObjectName(new_branch_info[1].clone()));
            }
        } else {
            commit_hash = match self.get_last_commit_hash()? {
                Some(hash) => hash,
                None => {
                    return Err(CommandError::InvalidObjectName(
                        self.get_current_branch_name()?,
                    ))
                }
            };
        }
        let new_branch = new_branch_info[0].clone();
        let branches_path = join_paths!(self.git_path, "refs/heads/").ok_or(
            CommandError::DirectoryCreationError(
                " No se pudo crear el directorio .git/refs/head/".to_string(),
            ),
        )?;

        let new_branch_path = branches_path.clone() + &new_branch;
        let path = Path::new(&new_branch_path);
        if path.exists() {
            return Err(CommandError::BranchExists(new_branch.clone()));
        }
        if path.is_dir() {
            return Err(CommandError::InvalidBranchName(new_branch));
        }
        if let Some(dir) = path.parent() {
            if !dir.ends_with(".git/refs/heads") {
                fs::create_dir_all(dir)
                    .map_err(|_| CommandError::FileCreationError(new_branch_path.clone()))?;
            }
        }
        self.log(&format!("Path: {}", new_branch_path));

        let mut file = File::create(new_branch_path.clone())
            .map_err(|_| CommandError::FileCreationError(new_branch_path.clone()))?;

        file.write_all(commit_hash.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(
                "Error guardando commit en la nueva rama:".to_string() + &error.to_string(),
            )
        })?;
        self.log(&format!("New branch created: {}", new_branch));
        if new_is_remote {
            writeln!(
                self.output,
                "Branch '{}' set up to track remote branch '{}'.",
                new_branch_info[1], new_branch_info[0]
            )
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }

    fn try_read_commit_local_branch(&self, branch: &str) -> Result<Option<String>, CommandError> {
        let branch_path = join_paths!(self.git_path, "refs/heads/").ok_or(
            CommandError::DirectoryCreationError(
                " No se pudo crear el directorio .git/refs/head/".to_string(),
            ),
        )?;

        let branch_path = branch_path.clone() + &branch;
        if !Path::new(&branch_path).exists() {
            return Ok(None);
        }
        let hash = fs::read_to_string(branch_path)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        Ok(Some(hash))
    }

    fn try_read_commit_remote_branch(
        &self,
        remote_path: &str,
    ) -> Result<Option<String>, CommandError> {
        let mut rel_path: Vec<&str> = remote_path.split_terminator('/').collect();
        if !rel_path.contains(&"remotes") {
            rel_path.insert(0, "remotes");
        }
        if !rel_path.contains(&"refs") {
            rel_path.insert(0, "refs");
        }
        let branch_path = rel_path.join("/");
        let branch_path =
            join_paths!(self.git_path, branch_path).ok_or(CommandError::FileCreationError(
                format!(" No se pudo crear el archivo: {}", branch_path),
            ))?;

        if !Path::new(&branch_path).exists() {
            return Ok(None);
        }
        let hash = fs::read_to_string(branch_path)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        Ok(Some(hash))
    }

    fn try_read_commit(&mut self, hash: &str) -> Result<Option<String>, CommandError> {
        if self.db()?.read_object(hash, &mut self.logger).is_err() {
            return Ok(None);
        }
        Ok(Some(hash.to_string()))
    }

    fn change_branch_name(&self, old: &str, new: &str) -> Result<(), CommandError> {
        let branches_path = join_paths!(self.git_path, "refs/heads/").ok_or(
            CommandError::DirectoryCreationError(
                " No se pudo crear el directorio .git/refs/head/".to_string(),
            ),
        )?;
        let old_path_str = branches_path.clone() + old;
        let new_path_str = branches_path + new;

        let old_path = Path::new(&old_path_str);
        let new_path = Path::new(&new_path_str);

        if !old_path.exists() {
            return Err(CommandError::NoOldBranch(old.to_string()));
        }
        if new_path.exists() && new != old {
            return Err(CommandError::NewBranchExists(new.to_string()));
        }

        let hash = fs::read_to_string(old_path)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;

        fs::remove_file(old_path)
            .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;

        if let Some(dir) = old_path.parent() {
            if !dir.ends_with(".git/refs/heads") {
                fs::remove_dir_all(dir)
                    .map_err(|_| CommandError::RemoveDirectoryError(old_path_str.clone()))?;
            }
        }

        if let Some(dir) = new_path.parent() {
            if !dir.ends_with(".git/refs/heads") {
                fs::create_dir_all(dir)
                    .map_err(|_| CommandError::FileCreationError(new_path_str.clone()))?;
            }
        }

        let mut file = File::create(new_path.clone())
            .map_err(|_| CommandError::FileCreationError(new_path_str.clone()))?;
        file.write_all(hash.as_bytes())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    pub fn delete_branches(
        &mut self,
        branches: &Vec<String>,
        are_remotes: bool,
    ) -> Result<(), CommandError> {
        let rel_branch_path = if are_remotes { "remotes/" } else { "heads/" };
        let branch_path = join_paths!(self.git_path, "refs/", rel_branch_path).ok_or(
            CommandError::DirectoryCreationError(
                " No se pudo crear el directorio .git/refs/".to_string(),
            ),
        )?;
        let mut errors: String = String::new();
        let mut deletions = String::new();
        let mut config = Config::open(&self.git_path)?;
        for branch in branches {
            let path = branch_path.clone() + branch;
            let Ok(_) = File::open(path.clone()) else {
                if are_remotes {
                    errors += &format!("error: remote-tracking branch '{branch}' not found.\n")
                } else {
                    errors += &format!("error: branch '{branch}' not found.\n")
                }
                continue;
            };
            let hash = fs::read_to_string(path.clone())
                .map_err(|error| CommandError::FileReadError(error.to_string()))?;

            fs::remove_file(path.clone())
                .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;

            if let Some(dir) = Path::new(&path).parent() {
                self.log(&format!("Dir: {:?}", dir));
                let remote: Vec<&str> = branch.split_terminator("/").collect();
                if !dir.ends_with("refs/heads") && remote.len() != 2 {
                    self.log(&format!("Deleting dir: {:?}", dir));

                    fs::remove_dir_all(dir)
                        .map_err(|_| CommandError::RemoveDirectoryError(path.clone()))?;
                }
            }
            config.remove_domain(&format!("branch \"{}\"", branch));
            if are_remotes {
                deletions += &format!(
                    "Deleted remote-tracking branch {branch} (was {}).\n",
                    hash[..7].to_string()
                );
            } else {
                deletions += &format!("Deleted branch {branch} (was {}).\n", hash[..7].to_string());
            }
        }
        config.save()?;
        write!(self.output, "{}{}", errors, deletions)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    pub fn show_local_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing local branches ...");

        let list = self.list_local_branches()?;
        write!(self.output, "{}", list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn list_local_branches(&mut self) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, "refs/heads").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;

        let mut local_branches: Vec<String> = Vec::new();
        self.log("Before getting branches paths");
        get_branches_paths(&mut self.logger, &path, &mut local_branches, "", 3)?;
        self.log("After getting branches paths");

        local_branches.sort();
        let mut list = String::new();
        let current = self.get_current_branch_name()?;
        for branch in local_branches.iter() {
            if branch.to_string() == current {
                list += &format!("* {}\n", branch);
            } else {
                list += &format!("  {}\n", branch);
            }
        }

        Ok(list)
    }

    pub fn show_all_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing all branches ...");
        let local_list = self.list_local_branches()?;
        let remote_list = self.list_remote_branches(2)?;

        write!(self.output, "{}{}", local_list, remote_list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    pub fn show_remote_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing remote branches ...");

        let list = self.list_remote_branches(3)?;
        write!(self.output, "{}", list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn list_remote_branches(&mut self, i: usize) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, "refs/remotes").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;
        let mut remote_branches: Vec<String> = Vec::new();
        get_branches_paths(&mut self.logger, &path, &mut remote_branches, "", i)?;
        remote_branches.sort();
        let mut list = String::new();
        let current = self.get_current_branch_name()?;
        for branch in remote_branches.iter() {
            let parts: Vec<&str> = branch.split("/").collect();
            if parts.ends_with(&["HEAD"]).to_string() == current {
                if let Some(path_ref_remote) = self.get_remote_head_path()? {
                    list += &format!("  {} -> {}\n", branch, path_ref_remote);
                } else {
                    list += &format!("  {}\n", branch);
                }
            } else {
                list += &format!("  {}\n", branch);
            }
        }

        Ok(list)
    }

    fn get_remote_head_path(&mut self) -> Result<Option<String>, CommandError> {
        let mut branch = String::new();
        let path = join_paths!(&self.git_path, "refs/remote/HEAD").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches remotas".to_string(),
            ),
        )?;
        let Ok(mut head) = File::open(&path) else {
            return Ok(None);
        };

        if head.read_to_string(&mut branch).is_err() {
            return Err(CommandError::FileReadError(path.to_string()));
        }

        let branch = branch.trim();
        let Some(branch) = branch.split(" ").last() else {
            return Err(CommandError::HeadError);
        };
        let branch: Vec<&str> = branch.split("/").collect();
        let branch = branch[2..].to_vec();
        let remote = branch.join("/");
        Ok(Some(remote.to_string()))
    }

    // ----- Checkout -----

    fn branch_exists(&mut self, branch: &str) -> bool {
        if let Ok(locals) = self.local_branches() {
            if locals.contains_key(branch) {
                return true;
            }
        }
        if let Ok(remotes) = self.remote_branches() {
            if remotes.contains_key(branch) {
                return true;
            }
        }
        false
    }

    fn file_exists_in_index(&mut self, file: &str, index: &mut StagingArea) -> bool {
        if index.has_file_from_path(file) {
            return true;
        }
        false
    }

    pub fn update_files_or_checkout(
        &mut self,
        files_or_branches: Vec<String>,
    ) -> Result<(), CommandError> {
        self.log(&format!("Checkout args: {:?}", files_or_branches));

        if files_or_branches.len() == 1 && self.branch_exists(&files_or_branches[0]) {
            self.log("Switching to new branch");
            self.checkout(&files_or_branches[0])?;
            return Ok(());
        }
        let mut staging_area = self.staging_area()?;
        for path in files_or_branches.iter() {
            if !self.file_exists_in_index(path, &mut staging_area) {
                return Err(CommandError::UntrackedError(path.to_string()));
            }
        }
        let db = self.db()?;
        for path in files_or_branches.iter() {
            if let Some(hash) = staging_area.get_files().get(path) {
                let mut blob = db.read_object(hash, &mut self.logger)?;
                blob.restore(path, &mut self.logger)?;
            }
        }
        Ok(())
    }

    pub fn checkout(&mut self, branch: &str) -> Result<(), CommandError> {
        let current_branch = self.get_current_branch_name()?;
        if branch == current_branch {
            writeln!(self.output, "Already on '{}'", current_branch)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
            return Ok(());
        }
        let Some(actual_hash) = self.get_last_commit_hash()? else {
            return Err(CommandError::UntrackedError(current_branch));
        };
        let Some(mut last_commit) = self.get_last_commit_tree()? else {
            return Err(CommandError::UntrackedError(current_branch));
        };
        let index = self.staging_area()?;

        let current_changes = ChangesController::new(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            &mut self.logger,
            Some(last_commit.clone()),
            &index,
        )?;
        let untracked_files = current_changes.get_untracked_files_bis();
        let changes_not_staged = current_changes.get_changes_not_staged();
        let changes_not_staged = get_modified_paths(changes_not_staged);
        let changes_staged = current_changes.get_changes_to_be_commited();
        let staging_area = self.staging_area()?;
        let changes_staged = get_staged_paths_and_content(
            changes_staged,
            &staging_area,
            &mut self.db()?,
            &mut self.logger,
        )?;

        let merge_files = staging_area.get_unmerged_files();
        if !merge_files.is_empty() {
            let merge_conflicts: Vec<&String> = merge_files.keys().collect();
            self.get_checkout_merge_conflicts_output(merge_conflicts)?;
            return Ok(());
        }

        let (new_hash, mut new_tree) = self.get_checkout_branch_info(branch, &self.db()?)?;
        let local_new_files =
            add_local_new_files(untracked_files, &mut new_tree, &mut self.logger)?;
        let mut deletions: Vec<String> = Vec::new();
        let mut modifications: Vec<String> = Vec::new();
        let mut conflicts: Vec<String> = Vec::new();

        let (ancestor, _, _) = self.get_common_ansestor(&new_hash, &actual_hash)?;
        let Some(common_tree) = ancestor.get_tree() else {
            return Err(CommandError::ObjectNotTree);
        };

        let has_conflicts = self.checkout_restore(
            &mut last_commit,
            new_tree,
            &new_hash,
            branch,
            &mut deletions,
            &mut modifications,
            &mut conflicts,
            &mut common_tree.to_owned(),
            untracked_files,
            &changes_not_staged,
            &changes_staged,
        )?;
        if has_conflicts {
            return Ok(());
        }
        self.get_checkout_sucess_output(branch, local_new_files, deletions, modifications)?;
        Ok(())
    }

    fn set_checkout_local_conflicts(
        &mut self,
        conflicts: Vec<String>,
        untracked_files: &Vec<String>,
    ) -> Result<(), CommandError> {
        let mut untracked_conflicts: Vec<String> = Vec::new();
        let mut unstaged_conflicts: Vec<String> = Vec::new();
        for path in conflicts.iter() {
            self.log(&format!("There is a conflict. Path: {}", path));
            if untracked_files.contains(path) {
                untracked_conflicts.push(path.to_string());
            } else {
                unstaged_conflicts.push(path.to_string());
            }
        }
        let mut message = String::new();
        untracked_conflicts.sort();
        unstaged_conflicts.sort();
        if !untracked_conflicts.is_empty() {
            message += "error: The following untracked working tree files would be overwritten by checkout:\n"
        }
        for path in untracked_conflicts.iter() {
            message += &format!("\t{}\n", path);
        }
        if !untracked_conflicts.is_empty() {
            message +=
                "Please commit your changes or stash them before you switch branches.\nAborting\n"
        }
        if !unstaged_conflicts.is_empty() {
            message += "error: Your local changes to the following files would be overwritten by checkout:\n"
        }
        for path in unstaged_conflicts.iter() {
            message += &format!("\t{}\n", path);
        }
        if !unstaged_conflicts.is_empty() {
            message += "Please move or remove them before you switch branches.\nAborting\n"
        }
        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn get_checkout_merge_conflicts_output(
        &mut self,
        merge_conflicts: Vec<&String>,
    ) -> Result<(), CommandError> {
        let mut message = String::new();

        for path in merge_conflicts.iter() {
            message += &format!("{path}: needs\n");
        }
        message += &format!("error: you need to resolve your current index first\n");
        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn get_checkout_sucess_output(
        &mut self,
        branch: &str,
        new_files: Vec<String>,
        deletions: Vec<String>,
        modifications: Vec<String>,
    ) -> Result<(), CommandError> {
        let mut message = String::new();
        let mut changes: Vec<String> = Vec::new();
        changes.extend_from_slice(&new_files);
        changes.extend_from_slice(&deletions);
        changes.extend_from_slice(&modifications);
        changes.sort();
        for path in changes.iter() {
            if new_files.contains(path) {
                message += &format!("A\t{path}\n");
            } else if deletions.contains(path) {
                message += &format!("D\t{path}\n");
            } else {
                message += &format!("M\t{path}\n");
            }
        }
        message += &format!("Switched to branch '{branch}'\n");
        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        // output de status (divergencia)
        Ok(())
    }

    fn get_checkout_branch_info(
        &mut self,
        branch: &str,
        db: &ObjectsDatabase,
    ) -> Result<(String, Tree), CommandError> {
        let path = join_paths!(&self.git_path, "refs/heads/", branch)
            .ok_or(CommandError::UntrackedError(branch.to_string()))?;
        let hash =
            fs::read_to_string(path.clone()).map_err(|_| CommandError::FileReadError(path))?;
        let mut commit = db.read_object(&hash, &mut self.logger)?;
        let Some(commit) = commit.as_mut_commit() else {
            return Err(CommandError::ObjectTypeError);
        };
        let Some(tree) = commit.get_tree() else {
            return Err(CommandError::ObjectNotTree);
        };
        Ok((hash, tree.to_owned()))
    }

    fn checkout_restore(
        &mut self,
        last_tree: &mut Tree,
        mut source_tree: Tree,
        new_hash: &str,
        branch: &str,
        deletions: &mut Vec<String>,
        modifications: &mut Vec<String>,
        conflicts: &mut Vec<String>,
        common: &mut Tree,
        untracked_files: &Vec<String>,
        unstaged_files: &Vec<String>,
        staged: &HashMap<String, Vec<u8>>,
    ) -> Result<bool, CommandError> {
        self.log("Restoring files");

        let staging_files = self.staging_area()?.get_files();

        self.look_for_checkout_conflicts(
            &mut source_tree,
            common,
            conflicts,
            untracked_files,
            false,
            &staging_files,
        )?;

        self.look_for_checkout_conflicts(
            &mut source_tree,
            common,
            conflicts,
            unstaged_files,
            false,
            &staging_files,
        )?;

        let staged_paths: Vec<&String> = staged.keys().collect();
        let staged_paths: Vec<String> = staged_paths.iter().map(|x| x.to_string()).collect();

        self.look_for_checkout_conflicts(
            &mut source_tree,
            common,
            conflicts,
            &staged_paths,
            true,
            &staging_files,
        )?;

        let mut working_tree = build_working_tree(&self.working_dir_path)?;
        source_tree.look_for_checkout_deletions_conflicts(
            &mut working_tree,
            common,
            conflicts,
            &self.working_dir_path,
            &mut self.logger,
        )?;

        if !conflicts.is_empty() {
            self.set_checkout_local_conflicts(conflicts.to_owned(), untracked_files)?;
            return Ok(true);
        }

        let delete = source_tree.checkout_restore(
            &self.working_dir_path,
            &mut self.logger,
            deletions,
            modifications,
            conflicts,
            common,
            unstaged_files,
            staged,
        )?;

        if delete {
            source_tree = Tree::new(self.working_dir_path.clone());
        }

        remove_new_files_commited(
            &mut working_tree,
            last_tree,
            &mut source_tree,
            &mut self.logger,
            &self.working_dir_path,
        )?;
        remove_local_changes_from_tree(untracked_files, &mut source_tree, &mut self.logger);

        let mut staging_area = self.staging_area()?;
        staging_area.update_to_tree(&source_tree)?;
        staging_area.save()?;
        self.update_ref_head(branch)?;
        update_last_commit(new_hash)?;
        Ok(false)
    }

    fn update_ref_head(&self, path: &str) -> Result<(), CommandError> {
        let head_path = join_paths!(&self.git_path, "HEAD")
            .ok_or(CommandError::FileCreationError(".git/HEAD".to_string()))?;
        let mut file = File::create(head_path)
            .map_err(|_| CommandError::FileCreationError(".git/HEAD".to_string()))?;
        let head_ref = format!("ref: refs/heads/{path}");
        file.write_all(head_ref.as_bytes())
            .map_err(|_| CommandError::FileWriteError(".git/HEAD".to_string()))?;

        Ok(())
    }

    fn look_for_checkout_conflicts(
        &mut self,
        new_tree: &mut Tree,
        common: &mut Tree,
        conflicts: &mut Vec<String>,
        files: &Vec<String>,
        is_staged: bool,
        staging_files: &HashMap<String, String>,
    ) -> Result<(), CommandError> {
        for path in files.iter() {
            self.log(&format!("Buscando conflictos en: {}", path));
            let path = &join_paths!(self.working_dir_path, path).ok_or(
                CommandError::FileCreationError(
                    "No se pudo obtener el path del objeto".to_string(),
                ),
            )?;
            if !Path::new(path).exists() {
                continue;
            }
            let is_in_common_tree = common.has_blob_from_path(&path, &mut self.logger);
            match new_tree.get_object_from_path(path) {
                None => {
                    if is_in_common_tree {
                        // Added: conflicto xq el tree de otra rama no lo tiene
                        conflicts.push(path.to_string());
                    }
                }
                Some(mut object) => {
                    let new_content = object.content(None)?;

                    let actual_content = {
                        if is_staged {
                            let Some(hash) = staging_files.get(path) else {
                                return Err(CommandError::ObjectHashNotKnown);
                            };
                            let c = self.get_staged_file_content(hash)?;
                            c
                        } else {
                            let c = get_current_file_content(path)?;
                            c
                        }
                    };

                    if has_conflicts(
                        &path,
                        &actual_content,
                        &new_content,
                        common,
                        &mut self.logger,
                    )? {
                        conflicts.push(path.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn get_staged_file_content(&mut self, hash: &str) -> Result<Vec<u8>, CommandError> {
        let mut object = self.db()?.read_object(hash, &mut self.logger)?;

        Ok(object.content(None)?)
    }

    // ----- Ls-files -----

    pub fn ls_files(
        &mut self,
        cached: bool,
        deleted: bool,
        modified: bool,
        others: bool,
        stage: bool,
        unmerged: bool,
        files: Vec<String>,
    ) -> Result<(), CommandError> {
        let last_commit = self.get_last_commit_tree()?;
        let index = self.staging_area()?;
        self.log(&format!("ls-files args: cached: {}, deleted: {}, modified: {}, others: {}, stage: {}, unmerged: {}, files: {:?}", cached, deleted, modified, others, stage, unmerged, files));
        let changes_controller = ChangesController::new(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            &mut self.logger,
            last_commit,
            &index,
        )?;
        let mut others_list = Vec::<String>::new();
        let mut aux_list = Vec::<String>::new();

        if others {
            others_list.extend_from_slice(&changes_controller.get_untracked_files_bis());
        }

        let mut staged_list = changes_controller.get_staged_files();
        let modifications_list = changes_controller.get_modified_files_working_tree();
        let staging_area_conflicts = index.get_unmerged_files();
        let staging_area_files = index.get_files();

        let unmerged_modifications_list = changes_controller.get_modified_files_unmerged();
        self.add_unmerged_files_to_list(&mut staged_list, staging_area_conflicts.clone());

        if cached {
            aux_list.extend_from_slice(&staged_list);
        }
        if unmerged && !cached && !stage {
            let aux_str_list: Vec<&String> = aux_list
                .iter()
                .filter(|p| staging_area_conflicts.contains_key(p.to_owned()))
                .collect();
            aux_list = aux_str_list.iter().map(|p| p.to_string()).collect();
        }

        if modified {
            aux_list.extend_from_slice(&modifications_list);
        }

        if deleted {
            let deleted_list = changes_controller.get_deleted_files();
            aux_list.extend_from_slice(&deleted_list);
        }

        if !files.is_empty() {
            others_list = others_list
                .iter()
                .filter(|p| files.contains(p.to_owned()))
                .map(|p| p.to_string())
                .collect();

            aux_list = aux_list
                .iter()
                .filter(|p| files.contains(p.to_owned()))
                .map(|p| p.to_string())
                .collect();
        }

        aux_list.sort();
        let message: String;
        let extended_list = self.get_extended_ls_files_info(
            aux_list.clone(),
            staging_area_conflicts,
            staging_area_files,
            unmerged_modifications_list,
        )?;

        if stage || unmerged {
            message = self.get_extended_ls_files_output(others_list.clone(), extended_list);
        } else {
            message = self.get_normal_ls_files_output(others_list, extended_list);
        }
        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    fn add_unmerged_files_to_list(
        &mut self,
        staged_list: &mut Vec<String>,
        staging_area_conflicts: HashMap<String, (Option<String>, Option<String>, Option<String>)>,
    ) {
        for (path, _) in staging_area_conflicts.iter() {
            staged_list.push(path.to_string());
        }
    }

    fn get_normal_ls_files_output(
        &mut self,
        others: Vec<String>,
        extended_list: Vec<(Mode, String, usize, String)>,
    ) -> String {
        let mut message = String::new();
        for path in others {
            message += &format!("{}\n", path);
        }
        for (_, _, _, path) in extended_list {
            message += &format!("{}\n", path);
        }
        message
    }

    fn get_extended_ls_files_output(
        &mut self,
        others: Vec<String>,
        extended_list: Vec<(Mode, String, usize, String)>,
    ) -> String {
        let mut message = String::new();
        for path in others {
            message += &format!("{}\n", path);
        }
        for (mode, hash, stage_number, path) in extended_list {
            message += &format!("{} {} {}\t{}\n", mode, hash, stage_number, path);
        }
        message
    }

    fn get_extended_ls_files_info(
        &mut self,
        list: Vec<String>,
        staging_area_conflicts: HashMap<String, (Option<String>, Option<String>, Option<String>)>,
        staging_area_files: HashMap<String, String>,
        unmerged_modifications_list: Vec<String>,
    ) -> Result<Vec<(Mode, String, usize, String)>, CommandError> {
        let mut result = Vec::<(Mode, String, usize, String)>::new();
        let db = self.db()?;
        for path in list.iter() {
            let is_modified = if unmerged_modifications_list.contains(path) {
                true
            } else {
                false
            };

            if let Some(hash) = staging_area_files.get(path) {
                let object = db.read_object(hash, &mut self.logger)?;
                let mode = object.mode();
                result.push((mode, hash.to_string(), 0, path.to_string()));
            };

            if let Some((common, head, remote)) = staging_area_conflicts.get(path) {
                match (common, head, remote) {
                    (Some(common_hash), Some(head_hash), Some(remote_hash)) => {
                        let object = db.read_object(common_hash, &mut self.logger)?;
                        let mode = object.mode();
                        result.push((mode.clone(), common_hash.to_string(), 1, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                common_hash.to_string(),
                                1,
                                path.to_string(),
                            ));
                        }
                        result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        if is_modified {
                            result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        }
                        result.push((mode.clone(), remote_hash.to_string(), 3, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                remote_hash.to_string(),
                                3,
                                path.to_string(),
                            ));
                        }
                    }
                    (Some(common_hash), Some(head_hash), None) => {
                        let object = db.read_object(common_hash, &mut self.logger)?;
                        let mode = object.mode();
                        result.push((mode.clone(), common_hash.to_string(), 1, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                common_hash.to_string(),
                                1,
                                path.to_string(),
                            ));
                        }
                        result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        if is_modified {
                            result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        }
                    }
                    (Some(common_hash), None, Some(remote_hash)) => {
                        let object = db.read_object(common_hash, &mut self.logger)?;
                        let mode = object.mode();
                        result.push((mode.clone(), common_hash.to_string(), 1, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                common_hash.to_string(),
                                1,
                                path.to_string(),
                            ));
                        }
                        result.push((mode.clone(), remote_hash.to_string(), 3, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                remote_hash.to_string(),
                                3,
                                path.to_string(),
                            ));
                        }
                    }
                    (None, Some(head_hash), Some(remote_hash)) => {
                        let object = db.read_object(head_hash, &mut self.logger)?;
                        let mode = object.mode();
                        result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        if is_modified {
                            result.push((mode.clone(), head_hash.to_string(), 2, path.to_string()));
                        }
                        result.push((mode.clone(), remote_hash.to_string(), 3, path.to_string()));
                        if is_modified {
                            result.push((
                                mode.clone(),
                                remote_hash.to_string(),
                                3,
                                path.to_string(),
                            ));
                        }
                    }
                    _ => {}
                }
            };
        }
        Ok(result)
    }
}

pub fn get_current_file_content(path: &str) -> Result<Vec<u8>, CommandError> {
    let content =
        fs::read_to_string(path).map_err(|error| CommandError::FileReadError(error.to_string()))?;
    Ok(content.as_bytes().to_vec())
}

// ----- Checkout -----

fn remove_new_files_commited(
    working_tree: &mut Tree,
    head_tree: &mut Tree,
    new_tree: &mut Tree,
    logger: &mut Logger,
    path: &str,
) -> Result<(), CommandError> {
    let mut new_files = Vec::<String>::new();
    working_tree.get_new_blobs_from_tree(new_tree, &mut new_files, path, logger)?;
    for path in new_files.iter() {
        if !head_tree.has_blob_from_path(path, logger) {
            logger.log(&format!("Deleting ... {}", path));
            fs::remove_file(path.clone())
                .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;
        }
    }

    Ok(())
}

fn has_conflicts(
    path: &str,
    content: &Vec<u8>,
    new_content: &Vec<u8>,
    common: &mut Tree,
    logger: &mut Logger,
) -> Result<bool, CommandError> {
    let content_str: String = String::from_utf8_lossy(content).to_string();

    let new_content_str: String = String::from_utf8_lossy(new_content).to_string();

    let mut common_content_str: String = "".to_string();
    if let Some(mut common_object) = common.get_object_from_path(path) {
        let content_u8 = common_object.content(None)?;
        common_content_str = String::from_utf8_lossy(&content_u8).to_string();
    }

    let (_, has_conflicts) = merge_content(
        content_str.clone(),
        new_content_str,
        common_content_str,
        "",
        "",
    )?;

    Ok(has_conflicts)
}

fn remove_local_changes_from_tree(
    untracked_files: &Vec<String>,
    tree: &mut Tree,
    logger: &mut Logger,
) {
    for path in untracked_files.iter() {
        logger.log(&format!(
            "Removing untracked file {} from new branch tree",
            path
        ));
        tree.remove_object_from_path(path, logger);
    }
}

fn add_local_new_files(
    untracked_files: &Vec<String>,
    tree: &mut Tree,
    logger: &mut Logger,
) -> Result<Vec<String>, CommandError> {
    let mut added: Vec<String> = Vec::new();
    for path in untracked_files {
        if !tree.has_blob_from_path(path, logger) {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            let data = fs::read_to_string(path)
                .map_err(|_| CommandError::FileReadError(path.to_string()))?;

            let hash = &crate::utils::aux::get_sha1_str(data.as_bytes());
            tree.add_path_tree(logger, vector_path, current_depth, hash)?;
            added.push(path.to_string());
        }
    }
    Ok(added)
}

fn get_modified_paths(unstaged_changes: &HashMap<String, ChangeType>) -> Vec<String> {
    let unstaged_changes = sort_hashmap_and_filter_unmodified(unstaged_changes);
    let mut changes: Vec<String> = Vec::new();

    for (path, _) in unstaged_changes.iter() {
        changes.push(path.to_string());
    }
    changes
}

fn get_staged_paths_and_content(
    staged_changes: &HashMap<String, ChangeType>,
    staging_area: &StagingArea,
    db: &mut ObjectsDatabase,
    logger: &mut Logger,
) -> Result<HashMap<String, Vec<u8>>, CommandError> {
    let staged_changes = sort_hashmap_and_filter_unmodified(staged_changes);
    let mut changes: HashMap<String, Vec<u8>> = HashMap::new();

    for (path, change_type) in staged_changes.iter() {
        if let ChangeType::Deleted = change_type {
            continue;
        }
        if let Some(hash) = staging_area.get_files().get(path) {
            let mut object = db.read_object(hash, logger)?;
            let content = object.content(None)?;
            changes.insert(path.to_string(), content);
        }
    }
    let log: Vec<&String> = changes.iter().map(|(s, _)| s).collect();
    Ok(changes)
}

// ----- Branch -----

fn get_branches_paths(
    logger: &mut Logger,
    path: &str,
    branches: &mut Vec<String>,
    dir_path: &str,
    i: usize,
) -> Result<(), CommandError> {
    let branches_path = join_paths!(path, dir_path).ok_or(CommandError::DirectoryCreationError(
        "Error creando directorio de branches".to_string(),
    ))?;
    /* if branches_path.ends_with("/") {
        _ = branches_path.pop();
    } */
    let paths = fs::read_dir(branches_path.clone()).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error leyendo directorio de branches: {}",
            error.to_string()
        ))
    })?;
    for entry in paths {
        logger.log(&format!("Entry: {:?}", entry));

        let entry = entry.map_err(|error| {
            CommandError::FileReadError(format!(
                "Error leyendo directorio de branches: {}",
                error.to_string()
            ))
        })?;
        let entry_path = entry.path();
        let path_str = &get_path_name(entry_path.clone())?;
        let parts: Vec<&str> = path_str.split("/").collect();
        let name = parts[i..].to_vec();
        let name = name.join("/");

        if entry_path.is_file() {
            branches.push(name);
        } else {
            if let Some(last) = parts.last() {
                get_branches_paths(logger, &branches_path, branches, &last, i)?;
            }
        }
    }

    Ok(())
}

// ----- Merge -----

fn merge_trees(
    head_tree: &mut Tree,
    destin_tree: &mut Tree,
    common_tree: &mut Tree,
    head_name: &str,
    destin_name: &str,
    parent_path: &str,
    staging_area: &mut StagingArea,
    logger: &mut Logger,
) -> Result<Tree, CommandError> {
    let mut merged_tree = Tree::new("".to_string());
    let mut head_entries = head_tree.get_objects();
    let mut destin_entries = destin_tree.get_objects();
    let mut common_entries = common_tree.get_objects();
    let all_keys = head_entries
        .clone()
        .into_keys()
        .chain(destin_entries.clone().into_keys())
        .collect::<HashSet<String>>();
    for key in all_keys {
        let head_entry = match head_entries.get_mut(&key) {
            Some((_head_entry_hash, head_entry_opt)) => {
                let head_entry = head_entry_opt.to_owned().ok_or(CommandError::ShallowTree)?;
                Some(head_entry)
            }
            None => None,
        };
        let destin_entry = match destin_entries.get_mut(&key) {
            Some((_destin_entry_hash, destin_entry_opt)) => {
                let destin_entry = destin_entry_opt
                    .to_owned()
                    .ok_or(CommandError::ShallowTree)?;
                Some(destin_entry)
            }
            None => None,
        };
        let common_entry = common_entries.get_mut(&key);

        let joint_path = join_paths!(parent_path, key).ok_or(CommandError::JoiningPaths)?;

        match common_entry {
            Some((_common_entry_hash, common_entry_opt)) => {
                let common_entry = common_entry_opt
                    .to_owned()
                    .ok_or(CommandError::ShallowTree)?;
                match is_in_common(
                    head_entry,
                    destin_entry,
                    common_entry,
                    head_name,
                    destin_name,
                    &joint_path,
                    staging_area,
                    logger,
                )? {
                    Some(merged_object) => merged_tree.add_object(key, merged_object)?,
                    _ => {}
                }
            }
            None => {
                let object = is_not_in_common(
                    head_entry,
                    destin_entry,
                    head_name,
                    destin_name,
                    &joint_path,
                    staging_area,
                    logger,
                )?;
                merged_tree.add_object(key, object)?
            }
        }
    }
    Ok(merged_tree)
}

fn is_in_common(
    head_entry: Option<GitObject>,
    destin_entry: Option<GitObject>,
    mut common_entry: GitObject,
    head_name: &str,
    destin_name: &str,
    parent_path: &str,
    staging_area: &mut StagingArea,
    logger: &mut Logger,
) -> Result<Option<GitObject>, CommandError> {
    match (head_entry, destin_entry) {
        (Some(mut head_entry), Some(mut destin_entry)) => {
            match (head_entry.as_mut_tree(), destin_entry.as_mut_tree()) {
                (Some(mut head_tree), Some(mut destin_tree)) => {
                    let mut common_tree = match common_entry.as_mut_tree() {
                        Some(common_tree) => common_tree.to_owned(),
                        None => Tree::new("".to_string()),
                    };
                    let merged_subtree = merge_trees(
                        &mut head_tree,
                        &mut destin_tree,
                        &mut common_tree,
                        head_name,
                        destin_name,
                        parent_path,
                        staging_area,
                        logger,
                    )?;
                    let boxed_subtree = Box::new(merged_subtree);
                    return Ok(Some(boxed_subtree));
                }
                (_, _) => match (head_entry.as_mut_blob(), destin_entry.as_mut_blob()) {
                    (Some(head_blob), Some(destin_blob)) => {
                        if head_blob.get_hash_string()? == destin_blob.get_hash_string()? {
                            return Ok(Some(head_entry.to_owned()));
                        } else {
                            let mut common_blob = match common_entry.as_mut_blob() {
                                Some(common_blob) => common_blob.to_owned(),
                                None => Blob::new_from_content(vec![])?,
                            };
                            let (merged_blob, merge_conflicts) = merge_blobs(
                                head_blob,
                                destin_blob,
                                &mut common_blob,
                                head_name,
                                destin_name,
                                logger,
                            )?;
                            if merge_conflicts {
                                staging_area.add_unmerged_file(
                                    parent_path,
                                    Some(common_blob.get_hash_string()?),
                                    Some(head_blob.get_hash_string()?),
                                    Some(destin_blob.get_hash_string()?),
                                );
                            } else {
                                staging_area.add(parent_path, &head_blob.get_hash_string()?);
                            }
                            return Ok(Some(Box::new(merged_blob.to_owned())));
                        }
                    }
                    (_, _) => {
                        return Err(CommandError::MergeConflict("".to_string()));
                    }
                },
            }
        }
        (Some(mut head_entry), None) => {
            staging_area.add_unmerged_object(
                &mut common_entry,
                &mut head_entry,
                parent_path,
                true,
            )?;
            return Ok(Some(head_entry.to_owned()));
        }
        (None, Some(mut destin_entry)) => {
            staging_area.add_unmerged_object(
                &mut common_entry,
                &mut destin_entry,
                parent_path,
                false,
            )?;

            return Ok(Some(destin_entry.to_owned()));
        }
        (None, None) => return Ok(None),
    };
}

fn is_not_in_common(
    head_entry: Option<GitObject>,
    destin_entry: Option<GitObject>,
    head_name: &str,
    destin_name: &str,
    entry_path: &str,
    staging_area: &mut StagingArea,
    logger: &mut Logger,
) -> Result<GitObject, CommandError> {
    match (head_entry, destin_entry) {
        (Some(mut head_entry), Some(mut destin_entry)) => {
            match (head_entry.as_mut_tree(), destin_entry.as_mut_tree()) {
                (Some(mut head_tree), Some(mut destin_tree)) => {
                    let mut common_tree = Tree::new("".to_string());
                    let merged_subtree = merge_trees(
                        &mut head_tree,
                        &mut destin_tree,
                        &mut common_tree,
                        head_name,
                        destin_name,
                        entry_path,
                        staging_area,
                        logger,
                    )?;
                    let boxed_subtree = Box::new(merged_subtree);
                    return Ok(boxed_subtree);
                }
                (_, _) => match (head_entry.as_mut_blob(), destin_entry.as_mut_blob()) {
                    (Some(head_blob), Some(destin_blob)) => {
                        if head_blob.get_hash_string()? == destin_blob.get_hash_string()? {
                            return Ok(head_entry.to_owned());
                        } else {
                            let mut common_blob = Blob::new_from_content(vec![])?;
                            let (mut merged_blob, merge_conflicts) = merge_blobs(
                                head_blob,
                                destin_blob,
                                &mut common_blob,
                                head_name,
                                destin_name,
                                logger,
                            )?;
                            if merge_conflicts {
                                staging_area.add_unmerged_file(
                                    entry_path,
                                    Some(common_blob.get_hash_string()?),
                                    Some(head_blob.get_hash_string()?),
                                    Some(destin_blob.get_hash_string()?),
                                );
                            } else {
                                let hash_str = merged_blob.get_hash_string()?;
                                staging_area.add(entry_path, &hash_str)
                            }
                            return Ok(Box::new(merged_blob.to_owned()));
                        }
                    }
                    (_, _) => {
                        return Err(CommandError::MergeConflict("".to_string()));
                    }
                },
            }
        }
        (Some(mut head_entry), None) => {
            staging_area.add_object(&mut head_entry, entry_path)?;
            return Ok(head_entry.to_owned());
        }
        (None, Some(mut destin_entry)) => {
            staging_area.add_object(&mut destin_entry, entry_path)?;
            return Ok(destin_entry.to_owned());
        }
        (None, None) => return Err(CommandError::MergeConflict("".to_string())),
    };
}

fn merge_blobs(
    head_blob: &mut Blob,
    destin_blob: &mut Blob,
    common_blob: &mut Blob,
    head_name: &str,
    destin_name: &str,
    logger: &mut Logger,
) -> Result<(Blob, bool), CommandError> {
    let head_content = head_blob.content(None)?;
    let destin_content = destin_blob.content(None)?;
    let common_content = common_blob.content(None)?;
    let head_content_str =
        String::from_utf8(head_content).map_err(|_| CommandError::CastingError)?;
    let destin_content_str =
        String::from_utf8(destin_content).map_err(|_| CommandError::CastingError)?;
    let common_content_str =
        String::from_utf8(common_content).map_err(|_| CommandError::CastingError)?;
    logger.log(&format!(
        "Common content: {}, Head content: {}, Remote content: {}",
        common_content_str, head_content_str, destin_content_str
    ));
    let (merged_content_str, merge_conflicts) = merge_content(
        head_content_str,
        destin_content_str,
        common_content_str,
        head_name,
        destin_name,
    )?;
    logger.log(&format!("Hay merge conflict?: {}", merge_conflicts));
    let merged_content = merged_content_str.as_bytes().to_owned();
    let merged_blob = Blob::new_from_content(merged_content)?;
    Ok((merged_blob, merge_conflicts))
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
    let branch_path = join_paths!(".git", currect_branch)
        .ok_or(CommandError::FileOpenError(currect_branch.clone()))?;
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

fn _remote_branches(base_path: &str) -> Result<HashMap<String, String>, CommandError> {
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

fn _update_remote_branches(
    server: &mut GitServer,
    repository_path: &str,
    repository_url: &str,
    base_path: &str,
) -> Result<(), CommandError> {
    let (_head_branch, branch_remote_refs) = server
        .explore_repository_upload_pack(&("/".to_owned() + repository_path), repository_url)?;
    Ok(for (sha1, mut ref_path) in branch_remote_refs {
        ref_path.replace_range(0..11, "");
        _update_ref(&base_path, &sha1, &ref_path)?;
    })
}

fn _update_ref(base_path: &str, sha1: &str, ref_name: &str) -> Result<(), CommandError> {
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
