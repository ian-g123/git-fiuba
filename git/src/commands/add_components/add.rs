use crate::commands::command::{Command, ConfigAdderFunction};
use git_lib::git_repository::GitRepository;
use git_lib::logger::Logger;
use git_lib::objects::blob::Blob;
use git_lib::objects::git_object::GitObject;
use git_lib::objects::last_commit::get_commit_tree;
use git_lib::staging_area::StagingArea;
use git_lib::{command_errors::CommandError, objects::tree::Tree};
use git_lib::{git_repository, objects_database};
use std::{
    env,
    fs::{self, DirEntry, ReadDir},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Commando Add
pub struct Add {
    pathspecs: Vec<String>,
}

impl Command for Add {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "add" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        logger.log(&format!("Add args: {:?}", args));

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Add> {
        vec![Add::add_file_config]
    }
}

impl Add {
    fn new(args: &[String]) -> Result<Add, CommandError> {
        let mut add = Add::new_default();
        add.config(args)?;
        Ok(add)
    }

    fn new_default() -> Add {
        Add {
            pathspecs: Vec::<String>::new(),
        }
    }

    fn add_file_config(add: &mut Add, i: usize, args: &[String]) -> Result<usize, CommandError> {
        add.pathspecs.push(args[i].clone());
        Ok(i + 1)
    }

    fn is_in_last_commit(path: &str, commit_tree: &Option<Tree>, logger: &mut Logger) -> bool {
        if let Some(tree) = commit_tree {
            return tree.has_blob_from_path(path, logger);
        }
        false
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        repo.add(self.pathspecs.clone())
    }
}
