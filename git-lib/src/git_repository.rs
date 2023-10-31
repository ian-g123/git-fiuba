use std::{
    collections::HashMap,
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

use crate::{
    changes_controller_components::{
        format::Format, long_format::LongFormat, short_format::ShortFormat,
    },
    command_errors::CommandError,
    config::Config,
    join_paths,
    logger::Logger,
    objects::{
        author::Author,
        blob::Blob,
        commit_object::{write_commit_tree_to_database, CommitObject},
        git_object::{self, GitObject, GitObjectTrait},
        last_commit,
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
            self.db()?.write(&mut object)?;
            self.logger
                .log(&format!("Writen object to database in {:?}", hex_string));
        }
        let _ = writeln!(self.output, "{}", hex_string);

        Ok(())
    }

    pub fn add(&mut self, pathspecs: Vec<String>) -> Result<(), CommandError> {
        let last_commit = &self.get_last_commit_tree()?;
        let mut staging_area = StagingArea::open()?;
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
        let hex_str = self.db()?.write(&mut git_object)?;
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
        let last_commit_tree = self.get_last_commit_tree()?;
        if !staging_area.has_changes(&self.db()?, &last_commit_tree, &mut self.logger)? {
            self.logger.log("Nothing to commit");
            self.status_long_format(true)?;
            return Ok(());
        }

        let last_commit_hash = self.get_last_commit()?;

        let mut parents: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            parents.push(padre);
        }

        self.log("Creating Index tree");

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

        self.log("Index tree created");

        let commit: CommitObject =
            self.get_commit(&message, parents, staged_tree.to_owned(), reuse_commit_info)?;
        self.log("get_commit");

        let mut git_object: GitObject = Box::new(commit);

        if !dry_run {
            write_commit_tree_to_database(&self.db()?, &mut staged_tree, &mut self.logger)?;
            let commit_hash = self.db()?.write(&mut git_object)?;
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
        let mut other_commit = self.db()?.read_object(&commit_hash)?;
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
        // let remote_branches = self.remote_branches()?;
        let wants_commits = remote_branches.clone().into_values().collect();
        let haves_commits = self.local_branches()?.into_values().collect();
        self.log(&format!("wants {:#?}", wants_commits));
        self.log(&format!("haves {:#?}", haves_commits));
        let objects_decompressed_data =
            server.fetch_objects(wants_commits, haves_commits, &mut self.logger)?;
        for (obj_type, len, content) in objects_decompressed_data {
            self.log(&format!(
                "Saving object of type {} and len {}, with data {:?}",
                obj_type,
                len,
                String::from_utf8_lossy(&content)
            ));
            let mut git_object: GitObject =
                Box::new(ProtoObject::new(content, len, obj_type.to_string()));
            self.db()?.write(&mut git_object)?;
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

    pub fn merge(&mut self, commits: &Vec<String>) -> Result<(), CommandError> {
        let mut commits = commits.clone();
        // if commits.is_empty() || (commits.len() == 1 && commits[0] == "FETCH_HEAD") {
        if commits.is_empty() {
            self.log("Running merge_head");
            commits.push(self.get_fetch_head_branch_commit_hash()?);
        }
        match self.get_last_commit()? {
            Some(last_commit) => self.merge_commits(&last_commit, &commits),
            None => self.merge_fast_forward(&commits),
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
    pub fn get_last_commit(&mut self) -> Result<Option<String>, CommandError> {
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

    // /// Abre la base de datos en la carpeta . git/refs/remotes/origin y obtiene los hashes de los
    // /// commits de las branches remotos.\
    // /// Devuelve un HashMap con el formato: `{nombre_branch: hash_commit}`.
    // fn remote_branches(&mut self) -> Result<HashMap<String, String>, CommandError> {
    //     let mut branches = HashMap::<String, String>::new();
    //     let branches_path = join_paths!(&self.path, ".git/refs/remotes/origin/").ok_or(
    //         CommandError::DirectoryCreationError(
    //             "Error creando directorio de branches".to_string(),
    //         ),
    //     )?;
    //     // let branches_path = format!("{}/.git/refs/remotes/origin/", &self.path);
    //     let paths = fs::read_dir(branches_path).map_err(|error| {
    //         CommandError::FileReadError(format!(
    //             "Error leyendo directorio de branches: {}",
    //             error.to_string()
    //         ))
    //     })?;
    //     for path in paths {
    //         let path = path.map_err(|error| {
    //             CommandError::FileReadError(format!(
    //                 "Error leyendo directorio de branches: {}",
    //                 error.to_string()
    //             ))
    //         })?;
    //         let file_name = &path.file_name();
    //         let Some(file_name) = file_name.to_str() else {
    //             return Err(CommandError::FileReadError(
    //                 "Error leyendo directorio de branches".to_string(),
    //             ));
    //         };
    //         let mut file = fs::File::open(path.path()).map_err(|error| {
    //             CommandError::FileReadError(format!(
    //                 "Error leyendo directorio de branches: {}",
    //                 error.to_string()
    //             ))
    //         })?;
    //         let mut sha1 = String::new();
    //         file.read_to_string(&mut sha1).map_err(|error| {
    //             CommandError::FileReadError(format!(
    //                 "Error leyendo directorio de branches: {}",
    //                 error.to_string()
    //             ))
    //         })?;
    //         branches.insert(file_name.to_string(), sha1);
    //     }
    //     Ok(branches)
    // }

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
        // let fetch_head_path = format!("{}/.git/FETCH_HEAD", &self.path);
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
                    "Error obteniendo la rama en HEAD:".to_string()
                        + "No se pudo obtener el nombre de la rama",
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

    /// Es el merge feliz, donde no hay conflictos. Se reemplaza el working tree por el del commit
    /// del remoto.
    fn merge_fast_forward(&mut self, commits: &[String]) -> Result<(), CommandError> {
        self.log("Merge fast forward");
        self.set_head_branch_commit_to(&commits[0])?;

        let db = self.db()?;
        self.log("Database opened");
        self.log(&format!("Reading commit {}", commits[0]));
        let mut commit_box = db.read_object(&commits[0])?;
        self.log("Commit read");
        let commit = commit_box
            .as_commit_mut()
            .ok_or(CommandError::FileReadError(
                "Error leyendo FETCH_HEAD".to_string(),
            ))?;
        let working_tree = commit.get_tree().to_owned();

        self.restore_tree(working_tree)?;
        Ok(())
    }

    /// Guarda en el archivo de la rama actual el hash del commit que se quiere hacer merge.
    fn set_head_branch_commit_to(&mut self, commits: &str) -> Result<(), CommandError> {
        let branch = self.get_head_branch_path()?;
        let branch_path = join_paths!(self.path, ".git", branch).ok_or(
            CommandError::FileWriteError(format!("Error abriendo {}", branch)),
        )?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&branch_path)
            .map_err(|error| {
                CommandError::FileWriteError(format!(
                    "Error guardando {}: {}",
                    branch_path,
                    &error.to_string()
                ))
            })?;
        file.write_all(commits.as_bytes()).map_err(|error| {
            CommandError::FileWriteError("Error guardando:".to_string() + &error.to_string())
        })?;
        Ok(())
    }

    fn restore_tree(&mut self, mut source_tree: Tree) -> Result<(), CommandError> {
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
                    let hex_str = self.db()?.write(&mut git_object)?;
                    staging_area.add(path, &hex_str);
                }
            }
        }
        Ok(())
    }

    pub fn get_last_commit_tree(&mut self) -> Result<Option<Tree>, CommandError> {
        let Some(last_commit) = self.get_last_commit()? else {
            return Ok(None);
        };
        self.log(&format!("Last commit : {}", last_commit));

        let mut commit_box = self.db()?.read_object(&last_commit)?;
        if let Some(commit) = commit_box.as_commit_mut() {
            self.log(&format!(
                "Last commit content : {}",
                String::from_utf8_lossy(&commit.content()?)
            ));
            let tree = commit.get_tree();

            self.log(&format!(
                "tree content : {}",
                String::from_utf8_lossy(&(tree.to_owned().content()?))
            ));
            return Ok(Some(tree.to_owned()));
        }
        Ok(None)
    }

    fn merge_commits(
        &mut self,
        last_commit: &str,
        commits: &Vec<String>,
    ) -> Result<(), CommandError> {
        self.log("Running merge_commits");
        let (mut common, mut commit_head, commit_destin) =
            self.get_common_ansestor(&commits, last_commit)?;
        if common.get_hash()? == commit_head.get_hash()? {
            return self.merge_fast_forward(&commits);
        }
        Ok(())
    }

    fn get_common_ansestor(
        &mut self,
        commits: &Vec<String>,
        commit_head_str: &str,
    ) -> Result<(CommitObject, CommitObject, CommitObject), CommandError> {
        self.log("Get common ansestor inicio");
        self.log(&format!("commits: {:?}", commits));

        let mut commit_head = self
            .db()?
            .read_object(&commit_head_str)?
            .as_commit_mut()
            .ok_or(CommandError::FailedToFindCommonAncestor)?
            .to_owned();
        self.log(&format!(
            "commit_head_hash: {:?}",
            commit_head.get_hash_string()
        ));

        let commit_destin = self
            .db()?
            .read_object(&commits[0])?
            .as_commit_mut()
            .ok_or(CommandError::FailedToFindCommonAncestor)?
            .to_owned();

        let mut head_branch_commits: HashMap<String, CommitObject> = HashMap::new();
        head_branch_commits.insert(commit_head_str.to_string(), commit_head.clone());

        let mut destin_branch_commits: HashMap<String, CommitObject> = HashMap::new();
        destin_branch_commits.insert(commits[0].to_string(), commit_destin.clone());

        let mut head_branch_tips: Vec<CommitObject> = [commit_head.clone()].to_vec();
        let mut destin_branch_tips: Vec<CommitObject> = [commit_destin.clone()].to_vec();

        loop {
            self.log(&format!("head_branch_tips: {:?}", &head_branch_tips.len()));
            self.log(&format!(
                "destin_branch_tips: {:?}",
                &destin_branch_tips.len()
            ));
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
        &self,
        branch_tips: &mut Vec<CommitObject>,
        branch_commits: &mut HashMap<String, CommitObject>,
    ) -> Result<(), CommandError> {
        let mut new_branch_tips = Vec::<CommitObject>::new();
        for tip in branch_tips.iter() {
            let parents_hash = tip.get_parents();
            for parent_hash in parents_hash {
                let parent = self
                    .db()?
                    .read_object(&parent_hash)?
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
