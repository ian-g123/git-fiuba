use std::{
    env,
    fs::{self, DirEntry, ReadDir},
    io::{Read, Write},
    path::Path,
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        objects::{blob::Blob, git_object::GitObject},
        objects_database,
        staging_area::StagingArea,
    },
    logger::Logger,
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

        instance.run(stdin, output, logger)?;
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

    fn run(
        &self,
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        for pathspec in &self.pathspecs {
            if !Path::new(pathspec).exists() {
                return Err(CommandError::FileOpenError(format!(
                    "No existe el archivo o directorio: {:?}",
                    pathspec
                )));
            }
        }
        let mut staging_area = StagingArea::open()?;
        for pathspec in &self.pathspecs {
            self.run_for_path(pathspec, &mut staging_area, output, logger)?
        }
        staging_area.save()?;
        Ok(())
    }

    fn run_for_path(
        &self,
        path: &str,
        staging_area: &mut StagingArea,
        _output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let path = Path::new(path);
        let path_str = &get_path_str(path)?;

        if path.is_file() {
            run_for_file(path_str, staging_area, logger)?;
            return Ok(());
        } else {
            self.run_for_dir(path_str, staging_area, logger, _output)?;
        }
        Ok(())
    }

    fn run_for_dir(
        &self,
        path_str: &String,
        staging_area: &mut StagingArea,
        logger: &mut Logger,
        _output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        let read_dir = self.read_dir(logger, path_str)?;
        for entry in read_dir {
            match entry {
                Ok(entry) => self.try_run_for_path(entry, staging_area, _output, logger)?,
                Err(error) => {
                    logger.log(&format!("Error in entry: {:?}", error));
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
        &self,
        entry: DirEntry,
        staging_area: &mut StagingArea,
        _output: &mut dyn Write,
        logger: &mut Logger,
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
        logger.log(&format!("entry: {:?}", path_str));
        self.run_for_path(path_str, staging_area, _output, logger)?;
        Ok(())
    }

    fn read_dir(&self, logger: &mut Logger, path_str: &String) -> Result<ReadDir, CommandError> {
        match fs::read_dir(path_str) {
            Ok(read_dir) => Ok(read_dir),
            Err(error) => Err(CommandError::FileOpenError(error.to_string())),
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

pub fn run_for_file(
    path: &str,
    staging_area: &mut StagingArea,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let blob = Blob::new_from_path(path.to_string())?;
    let mut git_object: GitObject = Box::new(blob);
    let hex_str = objects_database::write(logger, &mut git_object)?;
    staging_area.add(path, &hex_str);
    Ok(())
}
