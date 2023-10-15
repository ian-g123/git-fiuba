use std::{
    env,
    fs::{self, DirEntry, ReadDir},
    io::{Error, Read, Write},
    path::Path,
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        hash_object_components::hash_object::HashObject,
        stagin_area::StagingArea,
    },
    logger::Logger,
};

/// Commando Add
pub struct Rm {
    pathspecs: Vec<String>,
    cached: bool,
    recursive: bool,
    force : bool,
}

impl Command for Rm {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "rm" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        logger.log(&format!("removing {:?}", args));
        instance.run(stdin, output, logger)?;
        logger.log(&format!("rm {:?}", args));

        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Rm> {
        vec![Rm::rm_file_config, Rm::rm_cached_config, Rm::rm_dir_config, Rm::rm_force_config]
    }
}

impl Rm {
    fn new(args: &[String]) -> Result<Rm, CommandError> {
        let mut rm = Rm::new_default();
        rm.config(args)?;
        Ok(rm)
    }

    fn new_default() -> Rm {
        Rm {
            pathspecs: Vec::<String>::new(),
            cached: false,
            recursive: false,
            force : false,
        }
    }

    fn rm_dir_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "-r" {
            return Err(CommandError::WrongFlag);
        }
        rm.recursive = true;
        Ok(i + 1)
    }

    fn rm_force_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--force" || args[i] != "-f" {
            return Err(CommandError::WrongFlag);
        }
        rm.force = true;
        Ok(i + 1)
    }

    fn rm_cached_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--cached" {
            return Err(CommandError::WrongFlag);
        }
        rm.cached = true;
        Ok(i + 1)
    }

    fn rm_file_config(rm: &mut Rm, i: usize, args: &[String]) -> Result<usize, CommandError> {
        rm.pathspecs.push(args[i].clone());
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
            self.verifies_directory(pathspec)?;
        }

        let mut staging_area = StagingArea::open()?;
        for pathspec in &self.pathspecs {
            self.run_for_path(pathspec, &mut staging_area, output, logger)?
        }
        staging_area.save()?;
        Ok(())
    }

    fn verifies_directory(&self, pathspec: &String) -> Result<(), CommandError> {
        let path = Path::new(pathspec);
        if path.is_dir() && !self.recursive {
            return Err(CommandError::NotRecursive(pathspec.clone()));
        }
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
            self.run_for_file(path_str, staging_area, logger)?;
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
                    logger.log(&format!("Error en entry: {:?}", error));
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
            Err(error) => {
                logger.log(&format!(
                    "Error en read_dir: {error} desde {:?}",
                    env::current_dir()
                ));
                Err(CommandError::FileOpenError(error.to_string()))
            }
        }
    }

    fn run_for_file(
        &self,
        path: &str,
        staging_area: &mut StagingArea,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut file =
            fs::File::open(path).map_err(|error| file_open_error_maper(error, logger))?;
        let mut content = Vec::<u8>::new();
        file.read_to_end(&mut content)
            .map_err(|error| file_open_error_maper(error, logger))?;

        let hash_object = HashObject::new("blob".to_string(), vec![], true, false);
        let (hash_hex, _) = hash_object.run_for_content(content)?;

        self.verifies_version_of_index(); //IMPLEMENTAR!

        if !self.force{
            self.remove_from_stagin_area(path, hash_hex, staging_area, logger)?;
        }

        if !self.cached {
            fs::remove_file(path).map_err(|error| file_removing_error_maper(error, logger))?;
        }

        Ok(())
    }

    fn remove_from_stagin_area(
        &self,
        path: &str,
        hash_hex: String,
        staging_area: &mut StagingArea,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        staging_area.is_in_staging_area(path, hash_hex.clone())?;

        staging_area.remove(path);

        logger.log(&format!("staging_area.rm({},{})", path, &hash_hex));
        Ok(())
    }

    /// Verifica que la versión del commit anterior sea la misma. Los archivos que se eliminan
    /// deben ser idénticos al últmo commit, y no se pueden haber agregado nuevas versiones
    /// de este al index."
    fn verifies_version_of_index(&self) {}
}

fn get_path_str(path: &Path) -> Result<String, CommandError> {
    let Some(path_str) = path.to_str() else {
        return Err(CommandError::FileOpenError(
            "No se pudo convertir el path a str".to_string(),
        ));
    };
    Ok(path_str.to_string())
}

fn file_open_error_maper(error: Error, logger: &mut Logger) -> CommandError {
    logger.log(&format!("Error al abrir el archivo: {:?}", error));
    CommandError::FileOpenError(error.to_string())
}

fn file_removing_error_maper(error: Error, logger: &mut Logger) -> CommandError {
    logger.log(&format!("Error al eliminar el archivo: {:?}", error));
    CommandError::FileRemovingError(error.to_string())
}