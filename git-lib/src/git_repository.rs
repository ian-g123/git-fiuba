use std::{
    collections::{HashMap, HashSet},
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{DateTime, Local};

use crate::{
    changes_controller_components::{
        format::Format, long_format::LongFormat, short_format::ShortFormat,
    },
    command_errors::CommandError,
    config::Config,
    diff_components::merge::merge_content,
    join_paths,
    logger::Logger,
    objects::{
        author::Author,
        blob::Blob,
        commit_object::{write_commit_tree_to_database, CommitObject},
        git_object::{self, GitObject, GitObjectTrait},
        proto_object::ProtoObject,
        tree::Tree,
    },
    objects_database::ObjectsDatabase,
    server_components::git_server::GitServer,
    staging_area::StagingArea,
    utils::{aux::get_name, super_string::u8_vec_to_hex_string},
};

pub struct GitRepository<'a> {
    path: String,
    logger: Logger,
    output: &'a mut dyn Write,
}

impl<'a> GitRepository<'a> {
    pub fn open(path: &str, output: &'a mut dyn Write) -> Result<GitRepository<'a>, CommandError> {
        if !Path::new(
            &join_paths!(path, ".git").ok_or(CommandError::DirectoryCreationError(
                "Error abriendo directorio .git".to_string(),
            ))?,
        )
        .exists()
        {
            return Err(CommandError::NotGitRepository);
        }
        let logs_path = &join_paths!(path, ".git/logs").ok_or(
            CommandError::DirectoryCreationError("Error creando directorio .git/logs".to_string()),
        )?;
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
        let logs_path = &join_paths!(path, ".git/logs").ok_or(
            CommandError::DirectoryCreationError("Error creando directorio .git/logs".to_string()),
        )?;
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
            join_paths!(path.to_string(), ".git").ok_or(CommandError::DirectoryCreationError(
                "Error creando arc".to_string(),
            ))?
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
        let path_complete = join_paths!(path, name).ok_or(CommandError::DirectoryCreationError(
            "Error creando directorio".to_string(),
        ))?;
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
            join_paths!(path, ".git").ok_or(CommandError::FileCreationError(
                "Error creando arc".to_string(),
            ))?
        };
        self.create_file(&path_aux, "HEAD".to_string(), branch_name)?;
        Ok(())
    }

    fn create_file(&self, path: &str, name: String, branch_name: &str) -> Result<(), CommandError> {
        if fs::create_dir_all(path).is_ok() {
            //let path_complete = format!("{}/{}", path, name);
            let path_complete = join_paths!(path, name).ok_or(CommandError::FileCreationError(
                "Error creando un archivo".to_string(),
            ))?;
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
            self.db()?.write(&mut object, false, &mut self.logger)?;
            self.logger
                .log(&format!("Writen object to database in {:?}", hex_string));
        }
        let _ = writeln!(self.output, "{}", hex_string);

        Ok(())
    }

    pub fn add(&mut self, pathspecs: Vec<String>) -> Result<(), CommandError> {
        let last_commit = &self.get_last_commit_tree()?;
        let mut staging_area = StagingArea::open(&self.path)?;
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
        let hex_str = self.db()?.write(&mut git_object, false, &mut self.logger)?;
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
        let mut staging_area = StagingArea::open(&self.path)?;
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
        let mut staging_area = StagingArea::open(&self.path)?;
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
        let mut staging_area = StagingArea::open(&self.path)?;
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
            if !self.is_untracked(path, &staging_area_files)? {
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
        let last_commit_tree = self.get_last_commit_tree()?;

        staging_area.remove_changes(&last_commit_tree, &mut self.logger)?;
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
        self.log("commit_priv");
        let last_commit_tree = self.get_last_commit_tree()?;
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
                staging_area.get_working_tree_staged(&mut self.logger)?
            } else {
                staging_area.get_working_tree_staged_bis(
                    &last_commit_tree,
                    &mut self.logger,
                    files.clone(),
                )?
            }
        };

        let commit: CommitObject =
            self.get_commit(&message, parents, staged_tree.to_owned(), reuse_commit_info)?;

        let mut git_object: GitObject = Box::new(commit);

        if !dry_run {
            write_commit_tree_to_database(&mut self.db()?, &mut staged_tree, &mut self.logger)?;
            let commit_hash = self.db()?.write(&mut git_object, false, &mut self.logger)?;
            update_last_commit(&commit_hash)?;
        } else {
            self.status_long_format(true)?;
        }

        if !quiet {
            // salida de commit
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
        let config = Config::open(&self.path)?;
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
        let mut other_commit = self.db()?.read_object(&commit_hash, &mut self.logger)?;
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
        let branch = self.get_current_branch_name()?;
        let long_format = LongFormat;
        let last_commit_tree = self.get_last_commit_tree()?;
        long_format.show(
            &self.db()?,
            &self.path,
            last_commit_tree,
            &mut self.logger,
            &mut self.output,
            &branch,
            commit_output,
        )
    }

    pub fn status_short_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let branch = self.get_current_branch_name()?;
        let short_format = ShortFormat;
        let last_commit_tree = self.get_last_commit_tree()?;
        short_format.show(
            &self.db()?,
            &self.path,
            last_commit_tree,
            &mut self.logger,
            &mut self.output,
            &branch,
            commit_output,
        )
    }

    pub fn open_config(&self) -> Result<Config, CommandError> {
        Config::open(&self.path)
    }

    pub fn update_remote(&self, url: String) -> Result<(), CommandError> {
        let mut config = self.open_config()?;
        config.insert("remote \"origin\"", "url", &url);
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

        let remote_branches =
            self.update_remote_branches(&mut server, &repository_path, &repository_url)?;
        let remote_reference = format!("{}:{}", address, repository_path);
        self.fetch_and_save_objects(&mut server, &remote_reference, remote_branches)?;
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
        remote_branches: HashMap<String, String>,
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
        for (obj_type, len, content) in objects_decompressed_data {
            self.log(&format!(
                "Saving object of type {} and len {}, with data {:?}",
                obj_type,
                len,
                String::from_utf8_lossy(&content)
            ));
            let mut git_object: GitObject =
                Box::new(ProtoObject::new(content, len, obj_type.to_string()));
            self.db()?.write(&mut git_object, false, &mut self.logger)?;
        }

        self.update_fetch_head(remote_branches, remote_reference)?;
        Ok(())
    }

    pub fn log(&mut self, content: &str) {
        self.logger.log(content);
    }

    pub fn pull(&mut self) -> Result<(), CommandError> {
        self.fetch()?;
        self.merge(&Vec::new())?;
        Ok(())
    }

    pub fn push(&mut self) -> Result<(), CommandError> {
        self.log("pushing updates");
        let (address, repository_path, repository_url) = self.get_remote_info()?;

        self.log(&format!(
            "Address: {}, repository_path: {}, repository_url: {}",
            address, repository_path, repository_url
        ));
        let mut server = GitServer::connect_to(&address)?;

        let ref_and_hash = self.push2(&mut server, &repository_path, &repository_url)?;

        // self.exists_in_db(&ref_and_hash)?;

        Ok(())
    }

    /// Obtenemos todas las referencias con los hashes donde apuntan cada una del servidor.\
    /// Devuelve un HashMap con el formato: `{nombre_branch: hash_commit}`.
    fn push2(
        &mut self,
        server: &mut GitServer,
        repository_path: &str,
        repository_url: &str,
    ) -> Result<HashMap<String, String>, CommandError> {
        self.log("Updating remote branches");

        let ref_and_hash = server
            .explore_repository_receive(&("/".to_owned() + repository_path), repository_url)?;

        self.log(&format!("branch_remote_refs: {:?}\n", ref_and_hash));

        Ok(ref_and_hash)
    }

    // fn exists_in_db(&mut self, ref_and_hash: &HashMap<String, String>) -> Result<(), CommandError> {
    //     let db = self.db()?;
    //     for (ref_name, hash) in ref_and_hash {
    //         if !db.exists(hash) {
    //             return Err(CommandError::ObjectNotFound(ref_name.to_string()));
    //         }
    //     }
    //     Ok(())
    // }

    pub fn merge(&mut self, commits: &Vec<String>) -> Result<(), CommandError> {
        let mut head_name = "HEAD".to_string();
        let mut destin_name = "origin".to_string();
        if commits.len() > 1 {
            return Err(CommandError::MergeMultipleCommits);
        }
        let (destin_commit, is_remote) = match commits.first() {
            Some(commit) if commit != "FETCH_HEAD" => (commit.to_owned(), false),
            _ => (self.get_fetch_head_branch_commit_hash()?, true),
        };

        match self.get_last_commit_hash()? {
            Some(last_commit) => {
                let mut destin_branch_name = "".to_string();
                let mut head_branch_name = "".to_string();
                for (branch_name, branch_hash) in self.remote_branches()? {
                    if branch_hash == destin_commit {
                        destin_branch_name = branch_name;
                        break;
                    }
                }
                for (branch_name, branch_hash) in self.local_branches()? {
                    if branch_hash == last_commit {
                        head_branch_name = branch_name;
                        break;
                    }
                }

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
            join_paths!(self.path, ".git/HEAD").ok_or(CommandError::DirectoryCreationError(
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
            join_paths!(&self.path, ".git", branch).ok_or(CommandError::DirectoryCreationError(
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

    fn local_branches(&mut self) -> Result<HashMap<String, String>, CommandError> {
        let mut branches = HashMap::<String, String>::new();
        let branches_path = join_paths!(&self.path, ".git/refs/heads/").ok_or(
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
            branches.insert(file_name.to_string(), sha1);
        }
        Ok(branches)
    }

    /// Abre la base de datos en la carpeta . git/refs/remotes/origin y obtiene los hashes de los
    /// commits de las branches remotos.
    /// Devuelve un HashMap con el formato: `{nombre_branch: hash_commit}`.
    fn remote_branches(&mut self) -> Result<HashMap<String, String>, CommandError> {
        let mut branches = HashMap::<String, String>::new();
        let branches_path = join_paths!(&self.path, ".git/refs/remotes/origin/").ok_or(
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
    ) -> Result<(HashMap<String, String>), CommandError> {
        self.log("Updating remote branches");
        let (_head_branch, branch_remote_refs) =
            server.explore_repository(&("/".to_owned() + repository_path), repository_url)?;
        self.log(&format!("branch_remote_refs: {:?}", branch_remote_refs));

        let mut remote_branches = HashMap::<String, String>::new();

        for (hash, mut branch_name) in branch_remote_refs {
            branch_name.replace_range(0..11, "");
            self.update_ref(&hash, &branch_name)?;
            remote_branches.insert(branch_name, hash);
        }

        Ok(remote_branches)
    }

    /// Actualiza la referencia de la branch con el hash del commit obtenido del servidor.
    fn update_ref(&mut self, sha1: &str, ref_name: &str) -> Result<(), CommandError> {
        let dir_path = join_paths!(&self.path, ".git/refs/remotes/origin/").ok_or(
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
        let fetch_head_path = join_paths!(&self.path, ".git/FETCH_HEAD").ok_or(
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
            .ok_or(CommandError::NoHeadCommit)?;

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
                hash, branch_name, &self.path
            );
            file.write_all(line.as_bytes()).map_err(|error| {
                CommandError::FileWriteError(
                    "Error guardando FETCH_HEAD:".to_string() + &error.to_string(),
                )
            })?;
        }
        Ok(())
    }

    fn get_head_branch_name(&mut self) -> Result<String, CommandError> {
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
            join_paths!(&self.path, ".git/FETCH_HEAD").ok_or(CommandError::JoiningPaths)?;

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
            join_paths!(&self.path, ".git/FETCH_HEAD").ok_or(CommandError::JoiningPaths)?;

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
        source_tree.restore(&self.path, &mut self.logger)?;
        let mut staging_area = self.staging_area()?;
        staging_area.update_to_tree(&source_tree)?;
        staging_area.save()?;
        Ok(())
    }

    fn restore_merge_conflict(&mut self, mut source_tree: Tree) -> Result<(), CommandError> {
        self.log("Restoring files");
        source_tree.restore(&self.path, &mut self.logger)?;
        Ok(())
    }

    fn db(&self) -> Result<ObjectsDatabase, CommandError> {
        ObjectsDatabase::new(&self.path)
    }

    /// Devuelve true si Git no reconoce el path pasado.
    fn is_untracked(
        &mut self,
        path: &str,
        staging_area: &HashMap<String, String>,
    ) -> Result<bool, CommandError> {
        let mut blob = Blob::new_from_path(path.to_string())?;
        let hash = &blob.get_hash_string()?;
        let (is_in_last_commit, name) = self.is_in_last_commit_from_hash(hash.to_owned())?;
        if staging_area.contains_key(path) || (is_in_last_commit && name == get_name(&path)?) {
            return Ok(false);
        }
        Ok(true)
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
            if entry_name.contains(".git") {
                continue;
            }
            if entry_path.is_dir() {
                self.save_entries(&entry_name, staging_area, files)?;
                return Ok(());
            } else {
                let blob = Blob::new_from_path(entry_name.to_string())?;
                let path = &entry_name[2..];
                if !self.is_untracked(path, files)? {
                    let mut git_object: GitObject = Box::new(blob);
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
        self.log("Last commit readed");
        if let Some(commit) = commit_box.as_commit_mut() {
            self.log(&format!(
                "Last commit content : {}",
                String::from_utf8_lossy(&commit.content(None)?)
            ));
            let tree = commit.get_tree();

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
            .as_commit_mut()
            .ok_or(CommandError::FailedToFindCommonAncestor)?
            .to_owned();
        let commit_destin = self
            .db()?
            .read_object(commit_destin_str, &mut self.logger)?
            .as_commit_mut()
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
                    .as_commit_mut()
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

    fn true_merge(
        &mut self,
        common: &mut CommitObject,
        head: &mut CommitObject,
        destin: &mut CommitObject,
        head_name: &str,
        destin_name: &str,
    ) -> Result<(), CommandError> {
        let mut common_tree = common.get_tree().to_owned();
        let mut head_tree = head.get_tree().to_owned();
        let mut destin_tree = destin.get_tree().to_owned();

        let mut staging_area = self.staging_area()?;
        // staging_area.update_to_conflictings(merged_files.to_owned(), unmerged_files.to_owned());
        staging_area.clear();
        let merged_tree = merge_trees(
            &mut head_tree,
            &mut destin_tree,
            &mut common_tree,
            head_name,
            destin_name,
            &self.path,
            &mut staging_area,
        )?;
        staging_area.save()?;

        let message = format!("Merge branch '{}' into {}", destin_name, head_name);
        if staging_area.has_conflicts() {
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

    /// Dado un path, crea el archivo correspondiente y escribe el contenido pasado.
    fn write_to_file(&self, relative_path: &str, content: &str) -> Result<(), CommandError> {
        let path_f = join_paths!(self.path, ".git", relative_path).ok_or(
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
        Ok(StagingArea::open(&self.path)?)
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

    fn read_file(&self, relative_path: &str) -> Result<String, CommandError> {
        let path = join_paths!(self.path, ".git", relative_path).ok_or(
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
        let path =
            join_paths!(self.path, ".git", relative_path).ok_or(CommandError::JoiningPaths)?;
        fs::remove_file(path).map_err(|error| {
            CommandError::FileWriteError(format!(
                "Error borrando archivo {}: {}",
                relative_path,
                error.to_string()
            ))
        })?;
        Ok(())
    }

    // ----- Branch -----

    fn change_current_branch_name(&mut self, new_name: &str) -> Result<(), CommandError> {
        let path = join_paths!(self.path, ".git/HEAD").ok_or(CommandError::FileCreationError(
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
        let mut commit_hash = String::new();
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
        let branches_path = join_paths!(self.path, ".git/refs/heads/").ok_or(
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
            fs::create_dir_all(dir)
                .map_err(|_| CommandError::FileCreationError(new_branch_path.clone()))?;
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
        let branch_path = join_paths!(self.path, ".git/refs/heads/").ok_or(
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
            join_paths!(self.path, ".git/", branch_path).ok_or(CommandError::FileCreationError(
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
        let branches_path = join_paths!(self.path, ".git/refs/heads/").ok_or(
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
            if !dir.ends_with(".git/refs/heads/") {
                fs::remove_dir_all(dir)
                    .map_err(|_| CommandError::RemoveDirectoryError(old_path_str.clone()))?;
            }
        }

        if let Some(dir) = new_path.parent() {
            if !dir.ends_with(".git/refs/heads/") {
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
        branches: Vec<String>,
        are_remotes: bool,
    ) -> Result<(), CommandError> {
        let rel_branch_path = if are_remotes { "remotes/" } else { "heads/" };
        let branch_path = join_paths!(self.path, ".git/refs/", rel_branch_path).ok_or(
            CommandError::DirectoryCreationError(
                " No se pudo crear el directorio .git/refs/".to_string(),
            ),
        )?;
        let mut errors: Vec<String> = Vec::new();
        let mut deletions: Vec<(String, String)> = Vec::new();
        for branch in branches {
            let path = branch_path.clone() + &branch;
            let Ok(_) = File::open(path.clone()) else {
                errors.push(branch);
                continue;
            };
            let hash = fs::read_to_string(path.clone())
                .map_err(|error| CommandError::FileReadError(error.to_string()))?;
            deletions.push((branch, hash));
            fs::remove_file(path.clone())
                .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;

            if let Some(dir) = Path::new(&path).parent() {
                if !dir.ends_with(".git/refs/heads/") || !dir.ends_with(".git/refs/remotes/") {
                    fs::remove_dir_all(dir)
                        .map_err(|_| CommandError::RemoveDirectoryError(path.clone()))?;
                }
            }
        }
        // delete from config
        // output segun sean remotas o no
        Ok(())
    }

    /*
    delete_locals: Vec<String>,
    delete_remotes: Vec<String>,
    show_remotes: bool,
    show_all: bool,
     */
}

fn merge_trees(
    head_tree: &mut Tree,
    destin_tree: &mut Tree,
    common_tree: &mut Tree,
    head_name: &str,
    destin_name: &str,
    parent_path: &str,
    staging_area: &mut StagingArea,
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
        let head_entry = head_entries.get_mut(&key);
        let destin_entry = destin_entries.get_mut(&key);
        let common_entry = common_entries.get_mut(&key);
        let joint_path = join_paths!(parent_path, key).ok_or(CommandError::JoiningPaths)?;
        match common_entry {
            Some(common_entry) => {
                match is_in_common(
                    head_entry,
                    destin_entry,
                    common_entry,
                    head_name,
                    destin_name,
                    &joint_path,
                    staging_area,
                )? {
                    Some(merged_object) => merged_tree.add_object(key, merged_object),
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
                )?;
                merged_tree.add_object(key, object)
            }
        }
    }
    Ok(merged_tree)
}

fn is_in_common(
    head_entry: Option<&mut GitObject>,
    destin_entry: Option<&mut GitObject>,
    common_entry: &mut GitObject,
    head_name: &str,
    destin_name: &str,
    parent_path: &str,
    staging_area: &mut StagingArea,
) -> Result<Option<GitObject>, CommandError> {
    match (head_entry, destin_entry) {
        (Some(head_entry), Some(destin_entry)) => {
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
        (Some(head_entry), None) => {
            staging_area.add_unmerged_object(common_entry, head_entry, parent_path, true)?;
            return Ok(Some(head_entry.to_owned()));
        }
        (None, Some(destin_entry)) => {
            staging_area.add_unmerged_object(common_entry, destin_entry, parent_path, false)?;

            return Ok(Some(destin_entry.to_owned()));
        }
        (None, None) => return Ok(None),
    };
}

fn is_not_in_common(
    head_entry: Option<&mut GitObject>,
    destin_entry: Option<&mut GitObject>,
    head_name: &str,
    destin_name: &str,
    entry_path: &str,
    staging_area: &mut StagingArea,
) -> Result<GitObject, CommandError> {
    match (head_entry, destin_entry) {
        (Some(head_entry), Some(destin_entry)) => {
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
        (Some(head_entry), None) => {
            staging_area.add_object(head_entry, entry_path)?;
            return Ok(head_entry.to_owned());
        }
        (None, Some(destin_entry)) => {
            staging_area.add_object(destin_entry, entry_path)?;
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

    let (merged_content_str, merge_conflicts) = merge_content(
        head_content_str,
        destin_content_str,
        common_content_str,
        head_name,
        destin_name,
    )?;
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
