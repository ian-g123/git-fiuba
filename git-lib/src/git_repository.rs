use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use chrono::{DateTime, Local};

use crate::{
    changes_controller_components::{
        changes_controller::ChangesController,
        changes_types::ChangeType,
        commit_format::CommitFormat,
        format::Format,
        long_format::{set_diverge_message, sort_hashmap_and_filter_unmodified, LongFormat},
        short_format::ShortFormat,
        working_tree::{build_working_tree, get_path_name},
    },
    command_errors::CommandError,
    config::Config,
    diff_components::merge::merge_content,
    join_paths,
    logger::Logger,
    objects::{
        author::Author,
        blob::Blob,
        commit_object::{
            sort_commits_ascending_date, sort_commits_descending_date,
            write_commit_tree_to_database, CommitObject,
        },
        git_object::{self, GitObject, GitObjectTrait},
        mode::Mode,
        proto_object::ProtoObject,
        tag_object::{self, TagObject},
        tree::Tree,
    },
    objects_database::ObjectsDatabase,
    server_components::git_server::GitServer,
    server_components::{
        history_analyzer::{get_analysis, rebuild_commits_tree},
        packfile_functions::make_packfile,
        packfile_object_type::PackfileObjectType,
    },
    staging_area_components::staging_area::StagingArea,
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

    pub fn print_error_merge_conflicts(&mut self) -> Result<String, CommandError> {
        let staging_area = self.staging_area()?;
        let merge_conflicts = staging_area.get_unmerged_files();
        let files_merge_conflict = merge_conflicts.keys().cloned().collect::<HashSet<String>>();

        let mut files_merge_conflicts_message = String::new();
        for file in files_merge_conflict {
            files_merge_conflicts_message.push_str(&format!("{} needs merge\n", file));
        }

        let message = format!(
            "{}\nYou must edit all merge conflicts and then\nmark them as resolved using git add",
            files_merge_conflicts_message
        );
        Ok(message)
    }

    pub fn rebase_abort(&mut self) -> Result<(), CommandError> {
        let rebase_merge_path = join_paths!(self.git_path, "rebase-merge").ok_or(
            CommandError::DirectoryCreationError("Error creando directorio".to_string()),
        )?;

        // obtenemos los commits done
        let commits_todo = self.get_commits_todo()?;
        let last_commit_hash = commits_todo[commits_todo.len() - 1].to_owned().0;

        let branch_path = self.get_branch_path_for_rebase()?;

        self.set_branch_commit_to(branch_path, &last_commit_hash)?;

        let mut binding = self
            .db()?
            .read_object(&last_commit_hash, &mut self.logger)?;
        let main_commit = binding
            .as_mut_commit()
            .ok_or(CommandError::DirectoryCreationError(
                "Error creando directorio".to_string(),
            ))?;

        let Some(source_tree) = main_commit.get_tree() else {
            return Err(CommandError::RebaseError(
                "No hay árbol commit en abort".to_string(),
            ))?;
        };
        let mut staging_area = self.staging_area()?;

        let biding = self.db()?;
        self.restore(source_tree.to_owned(), &mut staging_area, Some(biding))?;

        self.delete_file("MERGE_MSG")?;
        self.delete_file("MERGE_HEAD")?;
        self.delete_file("AUTO_MERGE")?;

        fs::remove_dir_all(&rebase_merge_path).map_err(|_| {
            CommandError::DirectoryCreationError("Error borrando directorio".to_string())
        })?;
        Ok(())
    }

    pub fn initialize_rebase(
        &mut self,
        topic_branch: String,
        main_branch: String,
    ) -> Result<(), CommandError> {
        let last_hash_topic_branch = self.get_last_commit_hash_branch(&topic_branch)?;
        let last_hash_main_branch = self.get_last_commit_hash_branch(&main_branch)?;

        let (mut common_ancestor, _, _) =
            self.get_common_ancestor(&last_hash_topic_branch, &last_hash_main_branch)?;

        let common_string_hash = common_ancestor.get_hash_string()?;
        let mut ancestor_hash_to_look_for = HashSet::new();
        ancestor_hash_to_look_for.insert(common_string_hash);

        let mut commits_main = self
            .get_commits_until_ancestor(last_hash_main_branch, ancestor_hash_to_look_for.clone())?;
        let mut commits_topic =
            self.get_commits_until_ancestor(last_hash_topic_branch, ancestor_hash_to_look_for)?;

        sort_commits_ascending_date(&mut commits_main);
        sort_commits_ascending_date(&mut commits_topic);

        let commits_todo = self.initialize_commits_todo(commits_main, commits_topic)?;
        let first_hash_commit_todo = &commits_todo[0].0;
        let mut first_commit_todo = self
            .db()?
            .read_object(&first_hash_commit_todo, &mut self.logger)?;
        let first_commit_todo =
            first_commit_todo
                .as_mut_commit()
                .ok_or(CommandError::DirectoryCreationError(
                    "Error creando directorio".to_string(),
                ))?;
        let commits_done = vec![(
            common_ancestor.get_hash_string()?,
            common_ancestor.get_message(),
        )];

        //self.checkout(&topic_branch, false)?;
        self.make_rebase_merge_directory(
            commits_todo,
            commits_done,
            first_commit_todo,
            Some(main_branch),
        )?;

        let path_to_branch = self.get_branch_path_for_rebase()?;
        let path_to_branch2 = path_to_branch.clone();

        let branch_name =
            path_to_branch2
                .split("/")
                .last()
                .ok_or(CommandError::DirectoryCreationError(
                    "Error creando directorio".to_string(),
                ))?;

        let topic_hash = self.get_last_commit_hash_branch(&topic_branch)?;
        //let topic_hash = topic_hash.ok_or(CommandError::RebaseContinueError)?;

        self.set_branch_commit_to(path_to_branch, &topic_hash)?;

        self.checkout(branch_name, false)?;

        self.rebase_continue()?;
        Ok(())
    }

    fn initialize_commits_todo(
        &mut self,
        mut commits_main: Vec<(CommitObject, Option<String>)>,
        commits_topic: Vec<(CommitObject, Option<String>)>,
    ) -> Result<Vec<(String, String)>, CommandError> {
        let mut verify = true;
        let mut commits_todo = Vec::new();
        for i in 0..commits_main.len() {
            if i < commits_topic.len() && verify {
                let Some(tree_main) = commits_main[i].0.get_tree() else {
                    return Err(CommandError::DirectoryCreationError(
                        "Error creando directorio".to_string(),
                    ));
                };
                let mut tree_main = tree_main.clone();
                let Some(tree_topic) = commits_topic[i].0.get_tree() else {
                    return Err(CommandError::DirectoryCreationError(
                        "Error creando directorio".to_string(),
                    ));
                };
                let mut tree_topic = tree_topic.clone();
                if tree_main.get_hash() == tree_topic.get_hash() {
                    continue;
                } else {
                    verify = false;
                }
            }
            commits_todo.push((
                commits_main[i].0.get_hash_string()?,
                commits_main[i].0.get_message(),
            ));
        }
        Ok(commits_todo)
    }

    fn get_commits_until_ancestor(
        &mut self,
        last_hash_commit: String,
        hash_to_look_for: HashSet<String>,
    ) -> Result<Vec<(CommitObject, Option<String>)>, CommandError> {
        let db = self.db()?;
        let mut commits_map_topic = HashMap::new();
        rebuild_commits_tree(
            &db,
            &last_hash_commit,
            &mut commits_map_topic,
            None,
            false,
            &hash_to_look_for,
            true,
            &mut self.logger(),
        )?;
        Ok(commits_map_topic.drain().map(|(_, v)| v).collect())
    }

    fn make_rebase_merge_directory(
        &mut self,
        commits_todo: Vec<(String, String)>, // Vec<(hash, message)> del commit
        commits_done: Vec<(String, String)>, // Vec<(hash, message)> del commit
        first_commit_todo: &mut CommitObject,
        branch_name_op: Option<String>,
    ) -> Result<(), CommandError> {
        // directorio rebase-merge
        let rebase_merge_path = join_paths!(self.git_path, "rebase-merge").ok_or(
            CommandError::DirectoryCreationError("Error creando directorio".to_string()),
        )?;
        if !fs::create_dir_all(&rebase_merge_path).is_ok() {
            return Err(CommandError::DirectoryCreationError(rebase_merge_path));
        }
        let git_path = self.git_path.clone();

        // archivo message
        write_file_with(
            &git_path,
            "rebase-merge/message",
            first_commit_todo.get_message(),
        )?;

        // archivo head-name
        if branch_name_op.is_some() {
            let branch_name = match branch_name_op {
                Some(branch_name) => branch_name,
                None => "".to_string(),
            };
            let head_name = format!("refs/heads/{}", branch_name);
            write_file_with(&git_path, "rebase-merge/head-name", head_name)?;
        }

        // archivo git-rebase-todo
        let mut pick_hash_message = String::new();
        for (hash, message) in &commits_todo {
            pick_hash_message.push_str(&format!("pick {} {}\n", hash, message));
        }
        write_file_with(&git_path, "rebase-merge/git-rebase-todo", pick_hash_message)?;

        // archivo done
        let mut pick_hash_message = String::new();
        for (hash, message) in commits_done {
            pick_hash_message.push_str(&format!("pick {} {}\n", hash, message));
        }
        write_file_with(&git_path, "rebase-merge/done", pick_hash_message)?;

        // archivo REBASE_HEAD
        let commit_hash = commits_todo[0].0.clone();
        write_file_with(&git_path, "rebase-merge/REBASE_HEAD", commit_hash)?;

        // archivo author-script
        let author = first_commit_todo.get_author();
        let author_str = author.name;
        let email = author.email;
        let timestamp = first_commit_todo.get_timestamp();
        let author_script = format!(
            "GIT_AUTHOR_NAME='{}'\nGIT_AUTHOR_EMAIL='{}'\nGIT_AUTHOR_DATE='{}'",
            author_str, email, timestamp
        );
        write_file_with(&git_path, "rebase-merge/author-script", author_script)?;
        Ok(())
    }

    fn rebase_continue(&mut self) -> Result<String, CommandError> {
        // se fija si hay conflictos
        if self.staging_area()?.has_conflicts() {
            return Err(CommandError::RebaseContinueError);
        }
        // obtenemos los commits todo y los commits done

        let mut commits_todo = self.get_commits_todo()?;

        let mut commits_done = self.get_commits_done()?;

        // si no hay commits todo, termina el rebase
        if commits_todo.len() == 0 {
            let path_to_branch = self.get_branch_path_for_rebase()?;
            let branch_name =
                path_to_branch
                    .split("/")
                    .last()
                    .ok_or(CommandError::DirectoryCreationError(
                        "Error creando directorio".to_string(),
                    ))?;
            // self.checkout(branch_name, false)?;
            let rebase_merge_path = join_paths!(self.git_path, "rebase-merge").ok_or(
                CommandError::DirectoryCreationError("Error creando directorio".to_string()),
            )?;
            fs::remove_dir_all(&rebase_merge_path).map_err(|_| {
                CommandError::DirectoryCreationError("Error borrando directorio".to_string())
            })?;
            return Ok(branch_name.to_string());
        }

        // si hay commits todo, hace el merge de los dos commits (el usuario tiene que resolverlos)
        let branch_path_main = self.get_branch_path_for_rebase()?;
        let branch_name_main = self.get_branch_name(branch_path_main)?;
        //let topic_hash = self.get_last_commit_hash()?;
        let topic_hash = self.get_last_commit_hash_branch(&branch_name_main)?;

        //let topic_hash = topic_hash.ok_or(CommandError::RebaseContinueError)?;
        let mut binding = self.db()?.read_object(&topic_hash, &mut self.logger)?;
        let topic_commit = binding
            .as_mut_commit()
            .ok_or(CommandError::DirectoryCreationError(
                "Error creando directorio".to_string(),
            ))?;
        let main_hash = &commits_todo[0].0;
        let mut binding = self.db()?.read_object(&main_hash, &mut self.logger)?;
        let main_commit = binding
            .as_mut_commit()
            .ok_or(CommandError::DirectoryCreationError(
                "Error creando directorio".to_string(),
            ))?;

        let ancestor_hash = &commits_done[commits_done.len() - 1].0;
        let mut binding = self.db()?.read_object(&ancestor_hash, &mut self.logger)?;
        let ancestor_commit =
            binding
                .as_mut_commit()
                .ok_or(CommandError::DirectoryCreationError(
                    "Error creando directorio".to_string(),
                ))?;
        self.merge_two_commits_rebase(topic_commit, main_commit, ancestor_commit)?;

        // si hay conflictos, el usuario los tiene que resolver
        if self.staging_area()?.has_conflicts() {
            return Err(CommandError::RebaseMergeConflictsError);
        }

        // si no hay conflictos, sigue con el rebase_continue
        commits_done.push(commits_todo.remove(0));

        // si no hay commits todo, termina el rebase
        if commits_todo.len() == 0 {
            let path_to_branch = self.get_branch_path_for_rebase()?;
            let branch_name =
                path_to_branch
                    .split("/")
                    .last()
                    .ok_or(CommandError::DirectoryCreationError(
                        "Error creando directorio".to_string(),
                    ))?;
            let rebase_merge_path = join_paths!(self.git_path, "rebase-merge").ok_or(
                CommandError::DirectoryCreationError("Error creando directorio".to_string()),
            )?;
            fs::remove_dir_all(&rebase_merge_path).map_err(|_| {
                CommandError::DirectoryCreationError("Error borrando directorio".to_string())
            })?;
            // self.checkout(branch_name, false)?;
            return Ok(branch_name.to_string());
        }

        self.make_rebase_merge_directory(commits_todo, commits_done, topic_commit, None)?;
        return self.rebase_continue();
    }

    fn get_commits_todo(&mut self) -> Result<Vec<(String, String)>, CommandError> {
        let commit_todo_path = self.git_path.clone() + "/rebase-merge/git-rebase-todo";
        Ok(get_commits_rebase_merge(&commit_todo_path)?)
    }

    fn get_commits_done(&mut self) -> Result<Vec<(String, String)>, CommandError> {
        let commit_done_path = self.git_path.clone() + "/rebase-merge/done";
        Ok(get_commits_rebase_merge(&commit_done_path)?)
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
                    staging_area.add(&self.working_dir_path, relative_path, &actual_hash_lc)?;
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
        let content = fs::read(join_paths!(self.working_dir_path, path).ok_or(
            CommandError::DirectoryCreationError("Error abriendo directorio".to_string()),
        )?)
        .map_err(|_| CommandError::FileOpenError(format!("No existe el archivo: {:?}", path)))?;
        let blob = Blob::new_from_content_and_path(content, path)?;
        let mut git_object: GitObject = Box::new(blob);
        let hex_str = self.db()?.write(&mut git_object, false, &mut self.logger)?;
        self.log(&format!("File {} (hash: {}) added to index", path, hex_str));
        staging_area.add(&self.working_dir_path, path, &hex_str)?;
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

    // ----- Commit -----

    /// Hace un commit con los files pasados al comando.
    pub fn commit_files(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        let mut staging_area = self.staging_area()?;
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

    /// Incluye en el commit la versión del working tree de todos los archivos conocidos por git.
    pub fn commit_all(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        let mut staging_area = self.staging_area()?;
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

    /// Crea un Commit.
    pub fn commit(
        &mut self,
        message: String,
        files: &Vec<String>,
        dry_run: bool,
        reuse_commit_info: Option<String>,
        quiet: bool,
    ) -> Result<(), CommandError> {
        self.log("Running commit");
        let mut staging_area = self.staging_area()?;
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

        for path in files.iter() {
            self.log(&format!("Updating: {}", path));
            if !Path::new(path).exists() {
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

    /// Devuelve true si hay un merge en proceso.
    fn is_merge(&self) -> Result<bool, CommandError> {
        let path = join_paths!(self.git_path, "MERGE_HEAD").ok_or(CommandError::FileNameError)?;
        if Path::new(&path).exists() {
            return Ok(true);
        }
        Ok(false)
    }

    // ----- Status -----

    /// Compara los commits entre la rama local y la remota.
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
                let (mut common, _, _) = self.get_common_ancestor(remote, &head)?;
                common.get_hash_string()?
            }
            _ => "".to_string(),
        };
        let (ahead, behind) =
            self.count_commits_ahead_and_behind(commit_remote, commit_head, &common_hash)?;
        Ok((true, ahead, behind))
    }

    /// Ejecuta el comando Status con Long Format.
    pub fn status_long_format(&mut self, commit_output: bool) -> Result<(), CommandError> {
        let index = self.staging_area()?;
        let branch = self.get_current_branch_name()?;
        let long_format = LongFormat;
        let last_commit_tree = self.get_last_commit_tree()?;
        let merge = self.is_merge()?;
        let diverge_info = self.get_commits_ahead_and_behind_remote(&branch)?;

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

    /// Ejecuta el comando Status con Short Format.
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

    /// Cuenta los commits en que divergen la rama local y la remota.
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

    // -----

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
        let mut server = GitServer::connect_to(&address, &mut self.logger)?;

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

        let mut server = GitServer::connect_to(&address, &mut self.logger)?;
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

        server.negociate_recieve_pack(hash_branch_status)?;

        let pack_file: Vec<u8> = make_packfile(commits_map)?;
        server.send_packfile(&pack_file)?;

        _ = server.get_response()?;

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

    pub fn merge(&mut self, args: &Vec<String>) -> Result<(), CommandError> {
        self.log(&format!("Merge args: {:?}", args));
        if args.len() > 1 {
            return Err(CommandError::MergeMultipleCommits);
        }
        let (head_name, destin_commit, destin_name, destin_type_str) = match args.first() {
            Some(commit) if commit != "FETCH_HEAD" => {
                let (destin_commit, destin_name, destin_type_str) =
                    self.get_hash_and_name(commit)?;
                let head_name = self.get_current_branch_name()?;
                (head_name, destin_commit, destin_name, destin_type_str)
            }
            _ => (
                "HEAD".to_string(),
                self.get_fetch_head_branch_commit_hash()?,
                "origin".to_string(),
                "".to_string(),
            ),
        };

        match self.get_last_commit_hash()? {
            Some(last_commit) => {
                self.log(&format!("Merging two commits ...",));
                self.merge_two_commits(
                    &last_commit,
                    &destin_commit,
                    &head_name,
                    &destin_name,
                    &destin_type_str,
                )
            }
            None => self.merge_fast_forward(&destin_commit),
        }
    }

    fn get_hash_and_name(
        &mut self,
        pseudo_commit: &String,
    ) -> Result<(String, String, String), CommandError> {
        if let Some((destin_commit, branch_name, type_str)) = self.search_branch(pseudo_commit)? {
            return Ok((destin_commit, branch_name, type_str));
        }
        if let Some((destin_commit, tag_or_branch_name, type_str)) =
            self.search_tag(pseudo_commit)?
        {
            return Ok((destin_commit, tag_or_branch_name, type_str));
        }
        Ok((
            pseudo_commit.to_owned(),
            pseudo_commit[..7].to_string(),
            "commit".to_string(),
        ))
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
        // BranchName, CommitHash
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
            if file_name == "HEAD" {
                continue;
            }
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

    pub fn get_branch_name(&mut self, branch_path: String) -> Result<String, CommandError> {
        let head_branch_name =
            branch_path
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

        let mut staging_area = self.staging_area()?;

        self.restore_merge_fastfoward(tree, &mut staging_area, Some(self.db()?))?;
        Ok(())
    }

    /// Guarda en el archivo de la rama actual el hash del commit que se quiere hacer merge.
    fn set_head_branch_commit_to(
        &mut self,
        merge_commit_hash_str: &str,
    ) -> Result<(), CommandError> {
        let branch_path = self.get_head_branch_path()?;
        self.write_to_internal_file(&branch_path, merge_commit_hash_str)?;
        Ok(())
    }

    fn set_branch_commit_to(
        &mut self,
        branch_path: String,
        merge_commit_hash_str: &str,
    ) -> Result<(), CommandError> {
        self.write_to_internal_file(&branch_path, merge_commit_hash_str)?;
        Ok(())
    }

    fn restore(
        &mut self,
        mut source_tree: Tree,
        staging_area: &mut StagingArea,
        db: Option<ObjectsDatabase>,
    ) -> Result<(), CommandError> {
        source_tree.restore(&self.working_dir_path, &mut self.logger, db)?;
        staging_area.flush_soft_files(&self.working_dir_path)?;
        staging_area.save()?;
        Ok(())
    }

    fn restore_merge_fastfoward(
        &mut self,
        mut source_tree: Tree,
        staging_area: &mut StagingArea,
        db: Option<ObjectsDatabase>,
    ) -> Result<(), CommandError> {
        source_tree.restore(&self.working_dir_path, &mut self.logger, db)?;
        staging_area.update_to_tree(&self.working_dir_path, &source_tree)?;
        staging_area.save()?;
        Ok(())
    }

    fn restore_merge_conflict(
        &mut self,
        mut source_tree: Tree,
        staging_area: &mut StagingArea,
    ) -> Result<(), CommandError> {
        let objects_database = self.db()?;
        source_tree.restore(
            &self.working_dir_path,
            &mut self.logger,
            Some(objects_database),
        )?;
        staging_area.flush_soft_files(&self.working_dir_path)?;

        staging_area.save()?;

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

    /// Guarda en el staging area el estado actual del working tree, sin tener en cuenta los archivos
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
            } else {
                let content = fs::read(join_paths!(self.working_dir_path, entry_path).ok_or(
                    CommandError::DirectoryCreationError(
                        "Error creando directorio de branches".to_string(),
                    ),
                )?)
                .map_err(|error| {
                    CommandError::FileReadError(format!(
                        "Error leyendo archivo {}: {}",
                        entry_name,
                        error.to_string()
                    ))
                })?;
                let blob = Blob::new_from_content_and_path(content, &entry_name)?;
                let path = &entry_name[2..];
                if !self.is_untracked(path, &staging_area)? {
                    let mut git_object: GitObject = Box::new(blob);
                    self.log(&format!("Adding {} to staging area", path));
                    let hex_str = self.db()?.write(&mut git_object, false, &mut self.logger)?;
                    staging_area.add(&self.working_dir_path, path, &hex_str)?;
                }
            }
        }
        Ok(())
    }

    pub fn get_last_commit_tree(&mut self) -> Result<Option<Tree>, CommandError> {
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
        destin_type: &str,
    ) -> Result<(), CommandError> {
        let (mut common, mut commit_head, mut commit_destin) =
            self.get_common_ancestor(&destin_commit, head_commit)?;

        let hash = common.get_hash_string()?;

        self.log(&format!(
            "Merging {} '{}' into {}",
            destin_type, destin_commit, head_commit
        ));
        self.log(&format!("Common: {}", hash));
        if common.get_hash()? == commit_head.get_hash()? {
            return self.merge_fast_forward(&destin_commit);
        }

        self.true_merge(
            &mut common,
            &mut commit_head,
            &mut commit_destin,
            &head_name,
            &destin_name,
            &destin_type,
        )
    }

    fn merge_two_commits_rebase(
        &mut self,
        topic_commit: &mut CommitObject,
        main_commit: &mut CommitObject,
        commit_ancestor: &mut CommitObject,
    ) -> Result<(), CommandError> {
        self.log("Running merge_commits");
        self.log("True merge");
        self.true_merge_rebase(commit_ancestor, topic_commit, main_commit)
    }

    fn true_merge_rebase(
        &mut self,
        commit_ancestor: &mut CommitObject,
        topic_commit: &mut CommitObject,
        main_commit: &mut CommitObject,
    ) -> Result<(), CommandError> {
        let mut ancestor_tree = commit_ancestor.get_tree_some_or_err()?.to_owned();
        let mut topic_tree = topic_commit.get_tree_some_or_err()?.to_owned();
        let mut main_tree = main_commit.get_tree_some_or_err()?.to_owned();
        let mut staging_area = self.staging_area()?;

        let main_name = format!(
            "{} {}",
            &main_commit.get_hash_string()?[0..7],
            &main_commit.get_message()
        );

        staging_area.clear();
        let objects_database = self.db()?;
        let working_dir_path = self.working_dir_path.clone();
        let merged_tree = merge_trees(
            &mut topic_tree,
            &mut main_tree,
            &mut ancestor_tree,
            "HEAD",
            &main_name,
            &working_dir_path,
            &mut staging_area,
            &mut self.logger(),
            &working_dir_path,
            &objects_database,
        )?;

        //staging_area.save()?;

        let message = main_commit.get_message();
        if staging_area.has_conflicts() {
            self.log(&format!(
                "Conflicts rebase {:?}",
                staging_area.get_unmerged_files()
            ));
            let mut boxed_tree: GitObject = Box::new(merged_tree.clone());
            let merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;

            self.write_to_internal_file("MERGE_MSG", &message)?;
            self.write_to_internal_file("AUTO_MERGE", &merge_tree_hash_str)?;
            self.write_to_internal_file("MERGE_HEAD", &topic_commit.get_hash_string()?)?;

            self.restore_merge_conflict(merged_tree, &mut staging_area)?;
        } else {
            let mut boxed_tree: GitObject = Box::new(merged_tree.clone());
            let _merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;
            let merge_commit = self.create_new_commit(
                message,
                [topic_commit.get_hash_string()?].to_vec(),
                merged_tree.clone(),
            )?;

            let mut boxed_commit: GitObject = Box::new(merge_commit.clone());
            let merge_commit_hash_str =
                self.db()?
                    .write(&mut boxed_commit, false, &mut self.logger)?;

            let branch_path = self.get_branch_path_for_rebase()?;
            self.set_branch_commit_to(branch_path, &merge_commit_hash_str)?;
            self.restore(merged_tree, &mut staging_area, None)?;
        }
        Ok(())
    }

    fn get_branch_path_for_rebase(&mut self) -> Result<String, CommandError> {
        let rebase_head_path = join_paths!(self.git_path, "rebase-merge/head-name").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;
        let Ok(mut rebase_head_file) = File::open(rebase_head_path) else {
            return Err(CommandError::FileReadError(
                "Error leyendo directorio de rebase-merge/head-name".to_string(),
            ));
        };
        let mut branch_name_for_rebase = String::new();
        rebase_head_file
            .read_to_string(&mut branch_name_for_rebase)
            .map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo directorio de branches: {:?}",
                    error.to_string()
                ))
            })?;
        Ok(branch_name_for_rebase)
    }

    /// Tries to continue from failed merged
    pub fn merge_continue_rebase(&mut self) -> Result<String, CommandError> {
        let (message, _, _) = self.get_failed_merge_info_rebase()?;
        let mut staging_area = self.staging_area()?;
        if staging_area.has_conflicts() {
            return Err(CommandError::UnmergedFiles);
        }
        let merge_tree = staging_area.get_working_tree_staged(self.logger())?;

        let mut boxed_tree: GitObject = Box::new(merge_tree.clone());
        let merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;

        let get_last_commit_hash = self
            .get_last_commit_hash()?
            .ok_or(CommandError::FailedToResumeMerge)?;
        let parents = [get_last_commit_hash].to_vec();

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

        let branch_path = self.get_branch_path_for_rebase()?;
        self.set_branch_commit_to(branch_path, &merge_commit_hash_str)?;

        let db = self.db()?;
        self.restore(merge_tree, &mut staging_area, Some(db))?;
        self.delete_file("MERGE_MSG")?;
        self.delete_file("MERGE_HEAD")?;
        self.delete_file("AUTO_MERGE")?;

        //ACTUALIZAR ARCHIVOSS

        let mut commits_todo = self.get_commits_todo()?;
        let mut commits_done = self.get_commits_done()?;
        commits_done.push(commits_todo.remove(0));

        // si no hay commits todo, termina el rebase
        if commits_todo.len() == 0 {
            let path_to_branch = self.get_branch_path_for_rebase()?;
            let branch_name =
                path_to_branch
                    .split("/")
                    .last()
                    .ok_or(CommandError::DirectoryCreationError(
                        "Error creando directorio".to_string(),
                    ))?;
            let rebase_merge_path = join_paths!(self.git_path, "rebase-merge").ok_or(
                CommandError::DirectoryCreationError("Error creando directorio".to_string()),
            )?;
            fs::remove_dir_all(&rebase_merge_path).map_err(|_| {
                CommandError::DirectoryCreationError("Error borrando directorio".to_string())
            })?;
            self.checkout(branch_name, false)?;
            return Ok(branch_name.to_string());
        }

        let first_commit_todo = commits_todo
            .first()
            .ok_or(CommandError::FailedToResumeMerge)?;

        let mut first_commit_todo = self
            .db()?
            .read_object(&first_commit_todo.0, &mut self.logger)?
            .as_mut_commit()
            .ok_or(CommandError::FailedToResumeMerge)?
            .to_owned();

        self.make_rebase_merge_directory(
            commits_todo.clone(),
            commits_done.clone(),
            &mut first_commit_todo,
            None,
        )?;
        return self.rebase_continue();
    }

    fn get_common_ancestor(
        &mut self,
        commit_destin_str: &str,
        commit_head_str: &str,
    ) -> Result<(CommitObject, CommitObject, CommitObject), CommandError> {
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
        destin_type: &str,
    ) -> Result<(), CommandError> {
        self.log("True merge");
        let mut common_tree = common.get_tree_some_or_err()?.to_owned();
        let mut head_tree = head.get_tree_some_or_err()?.to_owned();
        let mut destin_tree = destin.get_tree_some_or_err()?.to_owned();

        let mut staging_area = self.staging_area()?;

        // staging_area.update_to_conflictings(merged_files.to_owned(), unmerged_files.to_owned());
        staging_area.clear();
        let working_dir_path = self.working_dir_path.clone();
        let objects_database = self.db()?;
        let merged_tree = merge_trees(
            &mut head_tree,
            &mut destin_tree,
            &mut common_tree,
            head_name,
            destin_name,
            &working_dir_path,
            &mut staging_area,
            &mut self.logger,
            &working_dir_path,
            &objects_database,
        )?;

        let message = format!("Merge {} '{}' into {}", destin_type, destin_name, head_name);
        if staging_area.has_conflicts() {
            let mut boxed_tree: GitObject = Box::new(merged_tree.clone());
            let merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;

            self.write_to_internal_file("MERGE_MSG", &message)?;
            self.write_to_internal_file("AUTO_MERGE", &merge_tree_hash_str)?;
            self.write_to_internal_file("MERGE_HEAD", &destin.get_hash_string()?)?;

            self.restore_merge_conflict(merged_tree, &mut staging_area)?;
            Ok(())
        } else {
            self.log("Conflicts resolved!");
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

            self.restore(merged_tree, &mut staging_area, None)?;
            Ok(())
        }
    }

    /// Tries to continue from failed merged
    pub fn merge_continue(&mut self) -> Result<(), CommandError> {
        let (message, _, destin) = self.get_failed_merge_info()?;
        let mut staging_area = self.staging_area()?;
        if staging_area.has_conflicts() {
            return Err(CommandError::UnmergedFiles);
        }
        let merge_tree = staging_area.get_working_tree_staged(self.logger())?;

        // let mut boxed_tree: GitObject = Box::new(merge_tree.clone());
        // let merge_tree_hash_str = self.db()?.write(&mut boxed_tree, true, &mut self.logger)?;

        let get_last_commit_hash = self
            .get_last_commit_hash()?
            .ok_or(CommandError::FailedToResumeMerge)?;
        let parents = [get_last_commit_hash, destin].to_vec();
        let merge_tree = staging_area.get_working_tree_staged(&mut Logger::new_dummy())?;

        let merge_commit = self.create_new_commit(message, parents, merge_tree.clone())?;
        let mut boxed_commit: GitObject = Box::new(merge_commit.clone());
        let merge_commit_hash_str = self
            .db()?
            .write(&mut boxed_commit, true, &mut self.logger)?;
        self.set_head_branch_commit_to(&merge_commit_hash_str)?;
        self.restore(merge_tree, &mut staging_area, Some(self.db()?))?;
        self.delete_file("MERGE_MSG")?;
        self.delete_file("AUTO_MERGE")?;
        self.delete_file("MERGE_HEAD")?;
        Ok(())
    }

    fn staging_area(&mut self) -> Result<StagingArea, CommandError> {
        let staging_area = StagingArea::open(&self.git_path);

        self.log(&format!("Staging area open: {:?}", staging_area));
        Ok(staging_area?)
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

    fn get_failed_merge_info_rebase(&mut self) -> Result<(String, String, String), CommandError> {
        // let (Ok(message), Ok(merge_tree_hash_str)) =
        //     (self.read_file("MERGE_MSG"), self.read_file("MERGE_HEAD"))
        // else {
        //     return Err(CommandError::NoMergeFound);
        // };

        // Ok((message, merge_tree_hash_str))

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
    pub fn write_to_internal_file(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), CommandError> {
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
        changes_not_staged.extend(untracked_files_vec.iter().cloned());
        return Ok((changes_to_be_commited, changes_not_staged));
    }

    // ----- Branch -----

    /// Cambia la referencia de HEAD.
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

    /// Cambia el nombre de una rama.
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

    /// Devuelve el commit a partir del cual se creará la nueva rama.
    fn get_start_point(
        &mut self,
        new_branch_info: &Vec<String>,
    ) -> Result<(String, bool), CommandError> {
        let commit_hash: String;
        let mut new_is_remote = false;
        if new_branch_info.len() == 2 {
            if let Some(hash) = self.try_read_commit_local_branch(new_branch_info[1].clone())? {
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
        Ok((commit_hash, new_is_remote))
    }

    /// Crea una nueva rama.
    pub fn create_branch(&mut self, new_branch_info: &Vec<String>) -> Result<(), CommandError> {
        let (commit_hash, new_is_remote) = self.get_start_point(new_branch_info)?;
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

    /// Si <branch> es una rama local, lee el commit asociado y lo devuelve.
    fn try_read_commit_local_branch(
        &mut self,
        mut branch: String,
    ) -> Result<Option<String>, CommandError> {
        if branch == "HEAD" {
            branch = self.get_current_branch_name()?;
        }
        let mut rel_path: Vec<&str> = branch.split_terminator('/').collect();
        if !rel_path.contains(&"heads") && !rel_path.contains(&"refs") {
            rel_path.insert(0, "heads");
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

    /// Si <branch> es una rama remota, lee el commit asociado y lo devuelve.
    fn try_read_commit_remote_branch(
        &self,
        remote_path: &str,
    ) -> Result<Option<String>, CommandError> {
        let mut rel_path: Vec<&str> = remote_path.split_terminator('/').collect();
        if !rel_path.contains(&"remotes") && !rel_path.contains(&"refs") {
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

    /// Si <hash> existe en la base de datos y es un commit, lo devuelve.
    fn try_read_commit(&mut self, hash: &str) -> Result<Option<String>, CommandError> {
        let Ok(mut object) = self.db()?.read_object(hash, &mut self.logger) else {
            return Ok(None);
        };
        if object.as_mut_commit().is_none() {
            return Ok(None);
        }
        Ok(Some(hash.to_string()))
    }

    /// Cambia el nombre de la rama <old> por <new>
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

    /// Elimina las ramas pasadas. Si alguna no existe, devuelve error y continúa con el resto.
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
                let remote: Vec<&str> = branch.split_terminator("/").collect();
                if !dir.ends_with("refs/heads") && remote.len() != 2 {
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

    /// Enlista las ramas locales.
    pub fn show_local_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing local branches ...");

        let list = self.list_local_branches()?;
        write!(self.output, "{}", list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    /// Devuelve el mensaje con la lista de ramas locales.
    fn list_local_branches(&mut self) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, "refs/heads").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;

        let mut local_branches: HashMap<String, String> = HashMap::new();

        let parts: Vec<&str> = if self.git_path != ".git" {
            self.git_path.split("/").collect()
        } else {
            Vec::new()
        };

        get_refs_paths_and_hash(
            &mut self.logger,
            &path,
            &mut local_branches,
            "",
            3 + parts.len(),
        )?;
        let mut local_branches: Vec<&String> = local_branches.keys().collect();

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

    /// Enlista todas las ramas.
    pub fn show_all_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing all branches ...");
        let local_list = self.list_local_branches()?;

        let remote_list = self.list_remote_branches(2)?;

        write!(self.output, "{}{}", local_list, remote_list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    /// Enlista las ramas remotas.
    pub fn show_remote_branches(&mut self) -> Result<(), CommandError> {
        self.log("Showing remote branches ...");

        let list = self.list_remote_branches(3)?;
        write!(self.output, "{}", list)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    /// Devuelve el mensaje con la lista de ramas remotas.
    fn list_remote_branches(&mut self, i: usize) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, "refs/remotes").ok_or(
            CommandError::DirectoryCreationError(
                "Error creando directorio de branches".to_string(),
            ),
        )?;
        let mut remote_branches: HashMap<String, String> = HashMap::new();

        let parts: Vec<&str> = if self.git_path != ".git" {
            self.git_path.split("/").collect()
        } else {
            Vec::new()
        };

        get_refs_paths_and_hash(
            &mut self.logger,
            &path,
            &mut remote_branches,
            "",
            i + parts.len(),
        )?;
        let mut remote_branches: Vec<&String> = remote_branches.keys().collect();
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

    /// Obtiene el path de refs/remote/HEAD.
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

    /// Muestra información de divergencia entre la rama local y la remota.
    pub fn show_tracking_info(&mut self) -> Result<(), CommandError> {
        let branch = self.get_current_branch_name()?;
        let (diverge, ahead, behind) = self.get_commits_ahead_and_behind_remote(&branch)?;
        if diverge {
            let message = set_diverge_message(ahead, behind, &branch);
            writeln!(self.output, "{}", message.trim())
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }

        Ok(())
    }

    /// Devuelve true si la rama pasada existe en el repositorio.
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

    /// Devuelve true si el archivo pasado está en el index.
    fn file_exists_in_index(&mut self, file: &str, index: &mut StagingArea) -> bool {
        if index.has_file_from_path(file) {
            return true;
        }
        false
    }

    /// Dependiendo de los elementos del vector pasado, ejecuta una operación de checkout o actualiza
    /// los archivos.
    pub fn update_files_or_checkout(
        &mut self,
        files_or_branches: Vec<String>,
    ) -> Result<(), CommandError> {
        self.log(&format!("Checkout args: {:?}", files_or_branches));

        if files_or_branches.len() == 1 {
            let pseudo_branch = files_or_branches[0].clone();
            if self.search_tag(&pseudo_branch)?.is_some() {
                return Err(CommandError::FeatureNotImplemented(
                    "checkout to ditached HEAD state".to_string(),
                ));
            }
            if self.branch_exists(&pseudo_branch) {
                self.log("Switching to new branch");
                self.checkout(&pseudo_branch, true)?;
                return Ok(());
            }
        }

        let mut staging_area = self.staging_area()?;

        for path in files_or_branches.iter() {
            if !self.file_exists_in_index(path, &mut staging_area) {
                return Err(CommandError::UntrackedError(path.to_string()));
            }
        }
        for path in files_or_branches.iter() {
            let db = self.db()?;
            if let Some(hash) = staging_area.get_files().get(path) {
                let mut blob = db.read_object(hash, &mut self.logger)?;
                blob.restore(path, &mut self.logger, Some(db))?;
            }
        }
        Ok(())
    }

    /// Ejecuta el cambio de rama.
    pub fn checkout(&mut self, branch: &str, verbose: bool) -> Result<(), CommandError> {
        let current_branch = self.get_current_branch_name()?;
        if branch == current_branch && verbose {
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
        let staging_area = self.staging_area()?;

        let current_changes = ChangesController::new(
            &self.db()?,
            &self.git_path,
            &self.working_dir_path,
            &mut self.logger,
            Some(last_commit.clone()),
            &staging_area,
        )?;
        let untracked_files = current_changes.get_untracked_files_bis();
        let changes_not_staged = current_changes.get_changes_not_staged();
        let changes_not_staged = get_modified_paths(changes_not_staged);
        let changes_staged = current_changes.get_changes_to_be_commited();

        let changes_staged = get_staged_paths_and_content(
            changes_staged,
            &staging_area,
            &self.db()?,
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

        let (ancestor, _, _) = self.get_common_ancestor(&new_hash, &actual_hash)?;
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
        self.log("checkout_restore");
        if has_conflicts {
            return Ok(());
        }
        self.get_checkout_sucess_output(
            branch,
            local_new_files,
            deletions,
            modifications,
            verbose,
        )?;
        Ok(())
    }

    /// Si hay conflictos locales que impiden el cambio de rama, se muestra el mensaje correspondiente.
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

    /// Si hay conflictos de merge que impiden el cambio de rama, se muestra el mensaje correspondiente.
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

    /// Muestra el mensaje de éxito de Checkout.
    fn get_checkout_sucess_output(
        &mut self,
        branch: &str,
        new_files: Vec<String>,
        deletions: Vec<String>,
        modifications: Vec<String>,
        verbose: bool,
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
        if verbose {
            message += &format!("Switched to branch '{branch}'\n");
            write!(self.output, "{}", message)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }

    /// Obtiene el hash y tree del commit de la rama pasada.
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

    /// Cambia el estado del working tree por los archivos de la nueva rama. Intenta mantener los
    /// cambios locales.
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
        self.log("Checkout restoring files");
        let staging_files = self.staging_area()?.get_files();
        self.log(&format!("staging_files: {:?}", staging_files));
        self.log("AAAA");

        self.look_for_checkout_conflicts(
            &mut source_tree,
            common,
            conflicts,
            untracked_files,
            false,
            &staging_files,
        )?;
        self.log("ABC");

        self.look_for_checkout_conflicts(
            &mut source_tree,
            common,
            conflicts,
            unstaged_files,
            false,
            &staging_files,
        )?;
        self.log("BBBB");

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

        let working_dir_path = self.working_dir_path.clone();
        let objects_database = self.db()?;
        let delete = source_tree.checkout_restore(
            &working_dir_path,
            &mut self.logger,
            deletions,
            modifications,
            conflicts,
            common,
            unstaged_files,
            staged,
            &objects_database,
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

        staging_area.update_to_tree(&self.working_dir_path, &source_tree)?;
        staging_area.save()?;
        self.update_ref_head(branch)?;
        update_last_commit(new_hash)?;
        Ok(false)
    }

    /// Actualiza la referencia a HEAD.
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

    /// Compara los archivos de la rama actual con los de la nueva y el ancestro común, buscando
    /// conflictos entre las ramas.
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
            let path = &join_paths!(self.working_dir_path, path).ok_or(
                CommandError::FileCreationError(
                    "No se pudo obtener el path del objeto".to_string(),
                ),
            )?;
            if !Path::new(path).exists() {
                continue;
            }
            let is_in_common_tree = common.has_blob_from_path(&path, &mut self.logger);
            let db = self.db()?;
            match new_tree.get_object_from_path(path) {
                None => {
                    if is_in_common_tree {
                        // conflicto xq el tree de otra rama no lo tiene
                        conflicts.push(path.to_string());
                    }
                }
                Some(mut object) => {
                    let new_content = object.content(Some(&db))?;

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

    /// Lee el contenido de un archivo.
    fn get_staged_file_content(&mut self, hash: &str) -> Result<Vec<u8>, CommandError> {
        let mut object = self.db()?.read_object(hash, &mut self.logger)?;

        Ok(object.content(None)?)
    }

    // ----- Ls-files -----

    /// Ejecuta el comando Ls-files
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
        add_unmerged_files_to_list(&mut staged_list, staging_area_conflicts.clone());

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
            message = get_extended_ls_files_output(others_list.clone(), extended_list);
        } else {
            message = get_normal_ls_files_output(others_list, extended_list);
        }
        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    /// Obtiene información adicional de los archivos a enlistar: modo, hash, staged number y path.
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

    // ----- Tag -----

    /// Crea un tag con la información pasada. Si se usó el flag -a, lo escribe a la base de datos.
    pub fn create_tag(
        &mut self,
        name: &str,
        message: &str,
        object: Option<String>,
        write: bool,
        force: bool,
    ) -> Result<(), CommandError> {
        self.log(&format!(
            "Creating tag --> name: {}, message: {}, points to: {:?}, force: {}, write to database: {}",
            name, message, object, force, write
        ));
        let path = join_paths!(self.git_path, "refs/tags/", name).ok_or(
            CommandError::FileCreationError("Error creando path de tags".to_string()),
        )?;
        let output_message = Self::get_create_tag_message(&path, force, name)?;
        let mut db = self.db()?;
        let (mut tag, tag_ref) = self.create_tag_object(name, message, object, &db)?;
        let file_content = {
            if write {
                tag.get_hash_string()?
            } else {
                tag_ref
            }
        };
        let Ok(mut file) = File::create(&path) else {
            return Err(CommandError::FileOpenError(path));
        };
        file.write_all(file_content.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(
                "Error guardando objeto en la nueva tag:".to_string() + &error.to_string(),
            )
        })?;
        if write {
            let mut git_object: GitObject = Box::new(tag);
            let _ = db.write(&mut git_object, false, &mut self.logger)?;
        }
        write!(self.output, "{}", output_message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }

    /// Crea un TagObject.
    fn create_tag_object(
        &mut self,
        name: &str,
        message: &str,
        object: Option<String>,
        db: &ObjectsDatabase,
    ) -> Result<(TagObject, String), CommandError> {
        self.log("Creating tag object");
        let tag_ref = {
            if let Some(obj) = object {
                obj
            } else {
                if let Some(hash) = self.get_last_commit_hash()? {
                    hash
                } else {
                    return Err(CommandError::InvalidRef("HEAD".to_string()));
                }
            }
        };

        if !db.has_object(&tag_ref)? && !self.tag_exists(&tag_ref)? {
            return Err(CommandError::InvalidRef(tag_ref));
        }

        let (tag_object, tag_ref) = self.read_tag_object(&tag_ref, db)?;

        self.log("Tag object read");

        let tag_object_type = tag_object.type_str();
        let (tagger, timestamp, offset) = self.get_tagger_info()?;
        let tag = TagObject::new(
            name.to_string(),
            tag_ref.clone(),
            tag_object_type,
            message.to_string(),
            tagger,
            timestamp,
            offset,
        );
        Ok((tag, tag_ref))
    }

    fn tag_exists(&self, tag_name: &str) -> Result<bool, CommandError> {
        let path = join_paths!(self.git_path, "refs/tags/", tag_name).ok_or(
            CommandError::FileCreationError("Error creando path de tags".to_string()),
        )?;
        Ok(Path::new(&path).exists())
    }

    fn read_tag_object(
        &mut self,
        tag_ref: &str,
        db: &ObjectsDatabase,
    ) -> Result<(GitObject, String), CommandError> {
        match db.read_object(&tag_ref, &mut self.logger) {
            Ok(object) => return Ok((object, tag_ref.to_string())),
            Err(_) => {
                let path = join_paths!(self.git_path, "refs/tags/", tag_ref).ok_or(
                    CommandError::FileCreationError("Error creando path de tags".to_string()),
                )?;
                self.log(&format!("Tag path (read): {}", path));
                let tag_ref = fs::read_to_string(path)
                    .map_err(|error| CommandError::FileReadError(error.to_string()))?;
                self.log(&format!("Tag ref (read): {}", tag_ref));
                return self.read_tag_object(&tag_ref, db);
            }
        }
    }

    /// Devuelve el mensaje que se mostrará al crear el tag.
    fn get_create_tag_message(path: &str, force: bool, name: &str) -> Result<String, CommandError> {
        let exists = Path::new(&path).exists();
        if exists && !force {
            return Err(CommandError::TagAlreadyExists(name.to_string()));
        }

        let output_message = {
            if exists {
                let hash = fs::read_to_string(path.clone())
                    .map_err(|error| CommandError::FileReadError(error.to_string()))?;
                format!("Updated tag '{}' (was {})\n", name, hash[..6].to_string())
            } else {
                "".to_string()
            }
        };
        Ok(output_message)
    }

    /// Devuelve información sobre un Tagger, incluyendo el timestamp y offset del tag.
    fn get_tagger_info(&mut self) -> Result<(Author, i64, i32), CommandError> {
        let config = self.open_config()?;
        let Some(tagger_email) = config.get("user", "email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(tagger_name) = config.get("user", "name") else {
            return Err(CommandError::UserConfigurationError);
        };
        let tagger = Author::new(tagger_name, tagger_email);
        let datetime: DateTime<Local> = Local::now();
        let timestamp = datetime.timestamp();
        let offset = datetime.offset().local_minus_utc() / 60;
        Ok((tagger, timestamp, offset))
    }

    /// Recibe un vector con nombres de tags y las elimina. Si alguna no existe, devuelve error y
    /// continúa con el resto.
    pub fn delete_tags(&mut self, tags: &Vec<String>) -> Result<(), CommandError> {
        self.log(&format!("Deleting tags --> {:?}", tags));
        let tags_path = join_paths!(self.git_path, "refs/tags/").ok_or(
            CommandError::DirectoryCreationError(
                "No se pudo crear el directorio .git/refs/tags".to_string(),
            ),
        )?;
        let mut errors: String = String::new();
        let mut deletions = String::new();

        for tag in tags {
            let path = tags_path.clone() + tag;
            if !Path::new(&path).exists() {
                errors += &format!("error: tag '{tag}' not found.\n");
                continue;
            };
            let hash = fs::read_to_string(path.clone())
                .map_err(|error| CommandError::FileReadError(error.to_string()))?;

            fs::remove_file(path.clone())
                .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;

            deletions += &format!("Deleted tag {tag} (was {}).\n", hash[..7].to_string());
        }
        write!(self.output, "{}{}", errors, deletions)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    /// Lista los tags de refs/tags/
    pub fn list_tags(&mut self) -> Result<(), CommandError> {
        self.log("Listing tags...");
        let mut tags = self.get_tag_names()?;

        tags.sort();

        let mut message = String::new();
        for tag in tags {
            message += &format!("{}\n", tag);
        }

        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    /// Devuelve un vector con los nombres de tags.
    fn get_tag_names(&mut self) -> Result<Vec<String>, CommandError> {
        let mut tags: Vec<String> = Vec::new();
        let tags_path = join_paths!(self.git_path, "refs/tags/").ok_or(
            CommandError::DirectoryCreationError(
                "No se pudo crear el directorio .git/refs/tags".to_string(),
            ),
        )?;

        let Ok(paths) = fs::read_dir(tags_path.clone()) else {
            return Err(CommandError::FileReadError(tags_path));
        };
        for path in paths {
            let path = path.map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo directorio de tags: {}",
                    error.to_string()
                ))
            })?;
            let file_name = &path.file_name();
            let Some(tag_name) = file_name.to_str() else {
                return Err(CommandError::FileReadError(
                    "Error leyendo directorio de tags".to_string(),
                ));
            };

            tags.push(tag_name.to_string());
        }
        Ok(tags)
    }

    /// Devuelve una tag guardada en la base de datos.
    fn get_tag_object_hash(&mut self, tag_name: &str) -> Result<String, CommandError> {
        let path = join_paths!(self.git_path, "refs/tags/", tag_name).ok_or(
            CommandError::DirectoryCreationError(
                "No se pudo crear el directorio .git/refs/tags".to_string(),
            ),
        )?;
        let tag_hash = fs::read_to_string(path.clone())
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        self.log(&format!("Tag hash: {}", tag_hash));
        let Some(tag_object) = self.db()?.read_object(&tag_hash, self.logger())?.as_tag() else {
            return Ok(tag_hash);
        };
        Ok(tag_object.get_object_hash())
    }

    // ----- Show-ref -----

    /// Ejecuta el comando show-ref
    pub fn show_ref(
        &mut self,
        head: bool,
        heads: bool,
        remotes: bool,
        tags: bool,
        dereference: bool,
        (hash, digits): (bool, Option<usize>),
        refs: &Vec<String>,
    ) -> Result<(), CommandError> {
        let mut refs_list: Vec<(String, String)> = Vec::new();

        let mut ref_heads = get_refs("refs/heads", &self.git_path, &mut self.logger)?;
        let mut ref_remotes = get_refs("refs/remotes", &self.git_path, &mut self.logger)?;
        let mut ref_tags = get_refs("refs/tags", &self.git_path, &mut self.logger)?;

        add_packed_refs(
            &mut ref_heads,
            &mut ref_remotes,
            &mut ref_tags,
            &self.git_path,
        )?;

        if dereference {
            ref_tags = dereference_tags(&ref_tags, &self.db()?, &mut self.logger)?;
        }

        let ref_heads = hashmap_to_vec(ref_heads);
        let ref_remotes = hashmap_to_vec(ref_remotes);
        let ref_tags = hashmap_to_vec(ref_tags);

        if head {
            if let Some(hash) = self.get_last_commit_hash()? {
                refs_list.push(("HEAD".to_string(), hash));
            }
        }
        if heads {
            refs_list.extend_from_slice(&ref_heads);
        }
        if remotes {
            refs_list.extend_from_slice(&ref_remotes);
        }

        if tags {
            refs_list.extend_from_slice(&ref_tags);
        }

        if !refs.is_empty() {
            refs_list = filter_refs(refs, refs_list);
        }

        let message = get_show_ref_message(&refs_list, hash, digits);

        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    fn search_branch(
        &mut self,
        pseudo_commit: &str,
    ) -> Result<Option<(String, String, String)>, CommandError> {
        let branches = self.local_branches()?;
        for (branch, hash) in branches.iter() {
            if branch == pseudo_commit {
                return Ok(Some((
                    hash.to_string(),
                    branch.to_string(),
                    "branch".to_string(),
                )));
            }
        }
        Ok(None)
    }

    /// Busca una tag con el nombre tag_name y si lo encuentra devuelve una terna
    /// con el hash, el nombre y "branch" si apunta al último commit de una branch, o
    /// "tag" en caso contrario
    fn search_tag(
        &mut self,
        tag_name: &str,
    ) -> Result<Option<(String, String, String)>, CommandError> {
        let tags = self.get_tag_names()?;
        self.log(&format!("Tags: {:?}", tags));
        let branches = self.local_branches()?;
        if tags.contains(&tag_name.to_string()) {
            let tag_object_hash = self.get_tag_object_hash(tag_name)?;

            for (branch_name, commit_hash) in branches.iter() {
                if commit_hash == &tag_object_hash {
                    return Ok(Some((
                        commit_hash.to_string(),
                        branch_name.to_string(),
                        "branch".to_string(),
                    )));
                }
            }
            let mut tag_object = self.db()?.read_object(&tag_object_hash, &mut self.logger)?;
            if !tag_object.as_mut_commit().is_some() {
                return Err(CommandError::MergeTagNotCommit(tag_name.to_string()));
            };
            return Ok(Some((
                tag_object_hash,
                tag_name.to_string(),
                "tag".to_string(),
            )));
        }
        Ok(None)
    }

    // ----- Ls-tree -----

    /// Ejecuta el comando ls-tree en base a los flags usados.
    pub fn ls_tree(
        &mut self,
        tree_ish: &str,
        only_list_trees: bool,
        recursive: bool,
        show_tree_entries: bool,
        show_size: bool,
        only_name: bool,
    ) -> Result<(), CommandError> {
        self.log(&format!(
            "Ls-tree args --> tree_ish: {}, -d: {}, -r: {}, -t: {}, -s: {}, --name-only: {}",
            tree_ish, only_list_trees, recursive, show_tree_entries, show_size, only_name
        ));
        let db = self.db()?;
        let tree =
            if let Some(commit_hash) = self.try_read_commit_local_branch(tree_ish.to_string())? {
                get_tree_from_commit(
                    &commit_hash,
                    &db,
                    &mut self.logger,
                    CommandError::LsTreeErrorNotATree,
                )?
            } else if let Some(commit_hash) = self.try_read_commit(tree_ish)? {
                get_tree_from_commit(
                    &commit_hash,
                    &db,
                    &mut self.logger,
                    CommandError::LsTreeErrorNotATree,
                )?
            } else if let Some(commit_hash) = self.try_read_commit_remote_branch(tree_ish)? {
                get_tree_from_commit(
                    &commit_hash,
                    &db,
                    &mut self.logger,
                    CommandError::LsTreeErrorNotATree,
                )?
            } else if let Some(tree) = self.try_read_tree(tree_ish)? {
                tree
            } else if let Some(tree) = self.try_read_tag(tree_ish)? {
                tree
            } else {
                return Err(CommandError::LsTreeErrorNotATree);
            };

        let mut info: Vec<(String, Mode, String, String, usize)> = Vec::new();

        add_ls_tree_info(
            &self.working_dir_path,
            tree,
            &mut info,
            only_list_trees,
            recursive,
            show_tree_entries,
        )?;

        let message = get_ls_tree_message(&info, show_size, only_name);

        write!(self.output, "{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    /// Si <hash> existe en la base de datos y es un tree, lo devuelve.
    fn try_read_tree(&mut self, hash: &str) -> Result<Option<Tree>, CommandError> {
        let Ok(mut object) = self.db()?.read_object(hash, &mut self.logger) else {
            return Ok(None);
        };
        if let Some(tree) = object.as_mut_tree() {
            return Ok(Some(tree.to_owned()));
        }
        return Ok(None);
    }

    /// Si <tag_name> es una tag del repositorio, intenta devolver el Tree al que finalmente apunta.
    /// Si no es una tag, o la misma no apunta a un Tree o Commit, devuelve None.
    fn try_read_tag(&mut self, tag_name: &str) -> Result<Option<Tree>, CommandError> {
        let mut rel_path: Vec<&str> = tag_name.split_terminator('/').collect();
        if !rel_path.contains(&"tags") && !rel_path.contains(&"refs") {
            rel_path.insert(0, "tags");
        }
        if !rel_path.contains(&"refs") {
            rel_path.insert(0, "refs");
        }
        let tag_path = rel_path.join("/");
        let tag_path = join_paths!(self.git_path, tag_path).ok_or(
            CommandError::FileCreationError(format!(" No se pudo crear el archivo: {}", tag_path)),
        )?;
        self.log(&format!("Tag path: {}", tag_path));
        if !Path::new(&tag_path).exists() {
            return Ok(None);
        }
        let hash = fs::read_to_string(tag_path)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;

        let db = self.db()?;
        let tree = get_tree_from_tag(hash, &db, &mut self.logger, tag_name.to_string())?;

        Ok(Some(tree))
    }
}

// ----- Ls-tree -----

/// Obtiene el mensaje de salida del comando ls-tree. Depende de los flags: -l, --long, --name-only, --status-only.
fn get_ls_tree_message(
    info: &Vec<(String, Mode, String, String, usize)>,
    show_size: bool,
    only_name: bool,
) -> String {
    let mut message = String::new();
    for (path, mode, type_str, hash, size) in info.iter() {
        if only_name {
            message += &format!("{}\n", path);
        } else if show_size {
            if type_str == "blob" {
                message += &format!(
                    "{} {} {}{:>8}	{}\n",
                    mode.to_string(),
                    type_str,
                    hash,
                    size,
                    path,
                );
            } else {
                message += &format!(
                    "{} {} {}       -	{}\n",
                    mode.to_string(),
                    type_str,
                    hash,
                    path,
                );
            }
        } else {
            message += &format!("{} {} {}	{}\n", mode.to_string(), type_str, hash, path);
        }
    }
    message
}

/// Obtiene los objetos de un tree y guarda la información que necesita el comando ls-tree:
/// nombre o path, tipo, modo y tamaño.
/// * Si se usó el flag -r y hay otros objetos trees, también se enlistan sus datos, de forma
/// recursiva.
/// * Si se usó el flag -d, solo se guarda la información de los trees.
/// * Se se usó el flag -t, se incluyen los trees, incluso si se usa -r.
fn add_ls_tree_info(
    path: &str,
    tree: Tree,
    info: &mut Vec<(String, Mode, String, String, usize)>,
    only_list_trees: bool,
    recursive: bool,
    show_tree_entries: bool,
) -> Result<(), CommandError> {
    for (name, (hash, opt_object)) in tree.sorted_objects() {
        let Some(mut object) = opt_object else {
            continue;
        };
        let full_path = join_paths!(path, name).ok_or(CommandError::DirectoryCreationError(
            "No se pudo crear el path del objeto".to_string(),
        ))?;
        let name = if !recursive { name } else { full_path.clone() };
        let hash_str = u8_vec_to_hex_string(&hash);
        let content = object.content(None)?;
        let type_str = object.type_str();

        if !((recursive && !show_tree_entries && type_str == "tree")
            || (only_list_trees && type_str == "blob"))
        {
            info.push((name, object.mode(), type_str, hash_str, content.len()));
        }
        if let Some(new_tree) = object.as_tree() {
            if recursive {
                add_ls_tree_info(
                    &full_path,
                    new_tree,
                    info,
                    only_list_trees,
                    recursive,
                    show_tree_entries,
                )?;
            }
        }
    }
    Ok(())
}

/// Dado el hash al que apunta un tag, intenta obtener el primer Tree en la cadena de referencias.
/// Si la tag finalmente apunta a un blob, devuelve error.
fn get_tree_from_tag(
    hash: String,
    db: &ObjectsDatabase,
    logger: &mut Logger,
    tag_name: String,
) -> Result<Tree, CommandError> {
    let mut object = db.read_object(&hash, logger)?;

    if let Some(tag) = object.as_mut_tag() {
        let hash = tag.get_object_hash();
        object = db.read_object(&hash, logger)?;
        if let Some(_) = object.as_mut_tag() {
            return get_tree_from_tag(hash, db, logger, tag_name);
        } else if let Some(tree) = object.as_mut_tree() {
            return Ok(tree.to_owned());
        } else {
            let tree = get_tree_from_commit(&hash, &db, logger, CommandError::LsTreeErrorNotATree)?;
            return Ok(tree);
        }
    } else if let Some(tree) = object.as_mut_tree() {
        return Ok(tree.to_owned());
    } else {
        let tree = get_tree_from_commit(&hash, &db, logger, CommandError::LsTreeErrorNotATree)?;
        return Ok(tree);
    }
}

/// Devuelve el tree del commit cuyo hash es pasado como parámetro.
fn get_tree_from_commit(
    commit_hash: &str,
    db: &ObjectsDatabase,
    logger: &mut Logger,
    error: CommandError,
) -> Result<Tree, CommandError> {
    let mut object = db.read_object(commit_hash, logger)?;
    let tree = if let Some(commit) = object.as_mut_commit() {
        let tree_hash = commit.get_tree_hash_string()?;
        let mut tree_object = db.read_object(&tree_hash, logger)?;
        if let Some(tree) = tree_object.as_mut_tree() {
            tree.to_owned()
        } else {
            return Err(error);
        }
    } else {
        return Err(error);
    };
    Ok(tree)
}

// ----- Show-ref -----

/// Devuelve el mensaje de salida del comando show-ref
fn get_show_ref_message(
    refs_list: &Vec<(String, String)>,
    show_hash: bool,
    digits_op: Option<usize>,
) -> String {
    let mut message = String::new();
    for (reference, hash) in refs_list {
        let final_hash = if let Some(digits) = digits_op {
            hash[..digits].to_string()
        } else {
            hash.to_owned()
        };
        if show_hash && !reference.ends_with("^{}") {
            message += &format!("{}\n", final_hash);
        } else {
            message += &format!("{} {}\n", final_hash, reference);
        }
    }
    message
}

/// Devuelve un diccionario con los objetos a los que referencian las tags: <tag_path, hash>
fn dereference_tags(
    tags: &HashMap<String, String>,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<HashMap<String, String>, CommandError> {
    let mut result: HashMap<String, String> = tags.clone();
    for (reference, hash) in tags {
        let mut git_object = db.read_object(hash, logger)?;
        let Some(tag) = git_object.as_mut_tag() else {
            continue;
        };
        let object_id = tag.get_object_hash();
        let name = reference.to_owned() + "^{}";
        _ = result.insert(name, object_id);
    }

    Ok(result)
}

/// Recibe un vector con nombres o paths de referencias y si son válidas devuelve una lista
/// con las referencias filtradas.
fn filter_refs(refs: &Vec<String>, list: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut refs_filtered: Vec<(String, String)> = Vec::new();
    for reference in refs {
        let ref_parts: Vec<&str> = reference.split("/").collect();
        let dereference = reference.to_owned() + "^{}";
        let deref_parts: Vec<&str> = dereference.split("/").collect();

        for (ref_str, hash) in list.iter() {
            let parts: Vec<&str> = ref_str.split("/").collect();
            if parts.ends_with(&ref_parts) || parts.ends_with(&deref_parts) {
                refs_filtered.push((ref_str.to_string(), hash.to_string()));
            }
        }
    }

    refs_filtered
}

/// Devuelve un diccionario con <ref, hash> dado el path de referencias pasado (heads, remotes o tags)
fn get_refs(
    path: &str,
    git_path: &str,
    logger: &mut Logger,
) -> Result<HashMap<String, String>, CommandError> {
    let path = join_paths!(git_path, path).ok_or(CommandError::DirectoryCreationError(
        "Error creando directorio de refs".to_string(),
    ))?;

    let mut refs: HashMap<String, String> = HashMap::new();

    let parts: Vec<&str> = if git_path != ".git" {
        git_path.split("/").collect()
    } else {
        Vec::new()
    };

    get_refs_paths_and_hash(logger, &path, &mut refs, "", 1 + parts.len())?;

    Ok(refs)
}

/// Agrega a las listas de referencias las que se encuentran en el archivo '.git/packed-refs'
fn add_packed_refs(
    heads: &mut HashMap<String, String>,
    remotes: &mut HashMap<String, String>,
    tags: &mut HashMap<String, String>,
    git_path: &str,
) -> Result<(), CommandError> {
    let packed_refs = read_packed_refs_file(git_path)?;
    for (reference, hash) in packed_refs.iter() {
        if reference.contains("refs/heads") && !heads.contains_key(reference) {
            _ = heads.insert(reference.to_string(), hash.to_string());
        }
        if reference.contains("refs/remotes") && !remotes.contains_key(reference) {
            _ = remotes.insert(reference.to_string(), hash.to_string());
        }
        if reference.contains("refs/tags") && !tags.contains_key(reference) {
            _ = tags.insert(reference.to_string(), hash.to_string());
        }
    }
    Ok(())
}

/// Convierte un hashmap a un vector de tuplas ordenado alfabéticamente.
fn hashmap_to_vec(refs: HashMap<String, String>) -> Vec<(String, String)> {
    let mut keys: Vec<&String> = refs.keys().collect();

    keys.sort();

    let mut sorted_refs: Vec<(String, String)> = Vec::new();
    for key in keys {
        if let Some(value) = refs.get(key) {
            sorted_refs.push((key.clone(), value.clone()));
        }
    }
    sorted_refs
}

/// Lee la lista de referencias del archivo '.git/packed-refs'
fn read_packed_refs_file(git_path: &str) -> Result<HashMap<String, String>, CommandError> {
    let mut refs: HashMap<String, String> = HashMap::new();
    let path = join_paths!(git_path, "packed-refs").ok_or(CommandError::DirectoryCreationError(
        "No se pudo crear el directorio .git/packed-refs".to_string(),
    ))?;
    if !Path::new(&path).exists() {
        return Ok(refs);
    }

    let content =
        fs::read_to_string(path).map_err(|error| CommandError::FileReadError(error.to_string()))?;

    let mut lines = content.lines();

    loop {
        let (eof, line) = next_line(&mut lines);
        if eof {
            break;
        }

        let Some((hash, ref_path)) = line.split_once(' ') else {
            return Err(CommandError::InvalidCommit);
        };
        if hash.len() != 40 {
            continue;
        }
        _ = refs.insert(ref_path.to_string(), hash.to_string());
    }

    Ok(refs)
}

/// Devuelve la próxima línea del iterador.
fn next_line(lines: &mut std::str::Lines<'_>) -> (bool, String) {
    let Some(line) = lines.next() else {
        return (true, "".to_string());
    };
    (false, line.to_string())
}

// ----- Ls-files -----

/// Agrega archivos no mergeados a la lista pasada.
fn add_unmerged_files_to_list(
    staged_list: &mut Vec<String>,
    staging_area_conflicts: HashMap<String, (Option<String>, Option<String>, Option<String>)>,
) {
    for (path, _) in staging_area_conflicts.iter() {
        staged_list.push(path.to_string());
    }
}

/// Obtiene el mensaje de salida de ls-files
fn get_normal_ls_files_output(
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

/// Obtiene el mensaje de salida de ls-files si se usa -s o -u.
fn get_extended_ls_files_output(
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

// ----- Checkout -----

/// Lee el contenido de un archivo.
pub fn get_current_file_content(path: &str) -> Result<Vec<u8>, CommandError> {
    let content =
        fs::read_to_string(path).map_err(|error| CommandError::FileReadError(error.to_string()))?;
    Ok(content.as_bytes().to_vec())
}

/// Elimina los archivos de la rama local que fueron commiteados y no están en la nueva rama.
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
        if head_tree.has_blob_from_path(path, logger) {
            fs::remove_file(path.clone())
                .map_err(|error| CommandError::RemoveFileError(error.to_string()))?;
        }
    }

    Ok(())
}

/// Compara el cotenido de las 3 versiones de un archivo (rama actual, nueva rama y ancestro) para determinar si hay conflictos.
fn has_conflicts(
    path: &str,
    content: &Vec<u8>,
    new_content: &Vec<u8>,
    common: &mut Tree,
    _logger: &mut Logger,
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

/// Elimina archivos nuevos de la rama actual del árbol que se usará para actualizar el index.
fn remove_local_changes_from_tree(
    untracked_files: &Vec<String>,
    tree: &mut Tree,
    logger: &mut Logger,
) {
    for path in untracked_files.iter() {
        tree.remove_object_from_path(path, logger);
    }
}

/// Agrega los archivos nuevos al tree que se usará para actualizar el working tree.
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
            let data = fs::read(path).map_err(|_| CommandError::FileReadError(path.to_string()))?;

            let hash = &crate::utils::aux::get_sha1_str(&data);
            let blob =
                Blob::new_from_hash_path_content_and_mode(hash, path, data, Mode::RegularFile)?;
            tree.add_path_tree(logger, vector_path, current_depth, blob)?;

            added.push(path.to_string());
        }
    }
    Ok(added)
}

/// Obtiene los archivos modificados en el working tree.
fn get_modified_paths(unstaged_changes: &HashMap<String, ChangeType>) -> Vec<String> {
    let unstaged_changes = sort_hashmap_and_filter_unmodified(unstaged_changes);
    let mut changes: Vec<String> = Vec::new();

    for (path, _) in unstaged_changes.iter() {
        changes.push(path.to_string());
    }
    changes
}

/// Obtiene los archivos del staging area y su contenido.
fn get_staged_paths_and_content(
    staged_changes: &HashMap<String, ChangeType>,
    staging_area: &StagingArea,
    db: &ObjectsDatabase,
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
    Ok(changes)
}

// ----- Branch -----

/// Obtiene el path de cada rama.
fn get_refs_paths_and_hash(
    logger: &mut Logger,
    path: &str,
    branches: &mut HashMap<String, String>,
    dir_path: &str,
    i: usize,
) -> Result<(), CommandError> {
    logger.log(&format!("indice: {}", i));
    let branches_path = join_paths!(path, dir_path).ok_or(CommandError::DirectoryCreationError(
        "Error creando directorio de branches".to_string(),
    ))?;

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
            let content = fs::read_to_string(path_str.clone()).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error leyendo directorio de branches: {}",
                    error.to_string()
                ))
            })?;
            branches.insert(name, content.trim().to_string());
        } else {
            if let Some(last) = parts.last() {
                get_refs_paths_and_hash(logger, &branches_path, branches, &last, i)?;
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
    working_dir: &str,
    db: &ObjectsDatabase,
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
                    working_dir,
                    db,
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
                    working_dir,
                    db,
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
    working_dir: &str,
    db: &ObjectsDatabase,
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
                        working_dir,
                        db,
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
                                db,
                            )?;
                            if merge_conflicts {
                                staging_area.soft_add_unmerged_file(
                                    working_dir,
                                    parent_path,
                                    Some(common_blob.get_hash_string()?),
                                    Some(head_blob.get_hash_string()?),
                                    Some(destin_blob.get_hash_string()?),
                                )?;
                            } else {
                                staging_area.soft_add(
                                    working_dir,
                                    parent_path,
                                    &head_blob.get_hash_string()?,
                                )?;
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
            staging_area.soft_add_unmerged_object(
                working_dir,
                &mut common_entry,
                &mut head_entry,
                parent_path,
                true,
            )?;
            return Ok(Some(head_entry.to_owned()));
        }
        (None, Some(mut destin_entry)) => {
            staging_area.soft_add_unmerged_object(
                working_dir,
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
    working_dir: &str,
    db: &ObjectsDatabase,
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
                        working_dir,
                        db,
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
                                db,
                            )?;
                            if merge_conflicts {
                                staging_area.soft_add_unmerged_file(
                                    working_dir,
                                    entry_path,
                                    Some(common_blob.get_hash_string()?),
                                    Some(head_blob.get_hash_string()?),
                                    Some(destin_blob.get_hash_string()?),
                                )?;
                            } else {
                                let hash_str = merged_blob.get_hash_string()?;
                                staging_area.soft_add(working_dir, entry_path, &hash_str)?
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
            staging_area.soft_add_object(working_dir, &mut head_entry, entry_path)?;
            return Ok(head_entry.to_owned());
        }
        (None, Some(mut destin_entry)) => {
            staging_area.soft_add_object(working_dir, &mut destin_entry, entry_path)?;
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
    db: &ObjectsDatabase,
) -> Result<(Blob, bool), CommandError> {
    let head_content = head_blob.content(Some(db))?;
    let destin_content = destin_blob.content(Some(db))?;
    let common_content = common_blob.content(Some(db))?;
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

fn write_file_with(git_path: &str, file_name: &str, content: String) -> Result<(), CommandError> {
    let file_path = join_paths!(git_path, file_name).ok_or(
        CommandError::DirectoryCreationError("Error creando archivo para stopped-sha".to_string()),
    )?;
    let mut archivo = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_path)
        .map_err(|_| CommandError::FileCreationError(file_path.to_string()))?;
    let _: Result<(), CommandError> = match archivo.write_all(content.as_bytes()) {
        Ok(_) => Ok(()),
        Err(err) => Err(CommandError::FileWriteError(err.to_string())),
    };
    Ok(())
}

fn get_commits_rebase_merge(commit_type_path: &str) -> Result<Vec<(String, String)>, CommandError> {
    let mut commits = Vec::new();
    let git_rebase_todo_path = join_paths!(commit_type_path).ok_or(
        CommandError::DirectoryCreationError("Error abriendo directorio".to_string()),
    );
    if let Ok(git_rebase_todo_path) = git_rebase_todo_path {
        let mut file = File::open(git_rebase_todo_path).map_err(|_| {
            CommandError::FileOpenError("Error abriendo archivo git_rebase_todo_path".to_string())
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| {
            CommandError::FileOpenError("Error leyendo archivo git_rebase_todo_path".to_string())
        })?;
        let lines: Vec<_> = contents.split('\n').collect();
        for line in lines {
            let line_vector: Vec<&str> = line.split(" ").collect();
            if line_vector.len() < 2 {
                break;
            }
            let hash = line_vector[1];
            let message = line_vector[2..].join(" ");

            commits.push((hash.to_string(), message.to_string()));
        }
    }
    Ok(commits)
}
