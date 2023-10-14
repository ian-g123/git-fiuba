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

        logger.log(&format!("adding {:?}", args));
        instance.run(stdin, output, logger)?;
        logger.log(&format!("add {:?}", args));
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
        for pathspec in &self.pathspecs {
            self.run_for_path(pathspec, output, logger)?
        }
        Ok(())
    }

    fn run_for_path(
        &self,
        path: &str,
        _output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let path = Path::new(path);
        let path_str = &get_path_str(path)?;

        if path.is_file() {
            run_for_file(path_str, logger)?;
            return Ok(());
        } else {
            self.run_for_dir(path_str, logger, _output)?;
        }
        Ok(())
    }

    fn run_for_dir(
        &self,
        path_str: &String,
        logger: &mut Logger,
        _output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        let read_dir = self.read_dir(logger, path_str)?;
        for entry in read_dir {
            match entry {
                Ok(entry) => self.try_run_for_path(entry, _output, logger)?,
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
        self.run_for_path(path_str, _output, logger)?;
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
}

fn get_path_str(path: &Path) -> Result<String, CommandError> {
    let Some(path_str) = path.to_str() else {
        return Err(CommandError::FileOpenError(
            "No se pudo convertir el path a str".to_string(),
        ));
    };
    Ok(path_str.to_string())
}

fn run_for_file(path: &str, logger: &mut Logger) -> Result<(), CommandError> {
    let mut file = fs::File::open(path).map_err(|error| file_open_error_maper(error, logger))?;
    let mut content = Vec::<u8>::new();
    file.read_to_end(&mut content)
        .map_err(|error| file_open_error_maper(error, logger))?;

    let hash_object = HashObject::new("blob".to_string(), vec![], true, false);
    let (hash_hex, _) = hash_object.run_for_content(content)?;
    save_to_stagin_area(path, hash_hex, logger)?;
    Ok(())
}

fn save_to_stagin_area(
    path: &str,
    hash_hex: String,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    Ok(match StagingArea::open() {
        Ok(mut staging_area) => {
            staging_area.add(path, &hash_hex);
            logger.log(&format!("staging_area.add({},{})", path, &hash_hex));
            staging_area.save()?;
        }
        Err(error) => {
            logger.log(&format!("Error al abrir el staging area: {:?}", error));
            return Err(CommandError::FailToOpenStaginArea(error.to_string()));
        }
    })
}

fn file_open_error_maper(error: Error, logger: &mut Logger) -> CommandError {
    logger.log(&format!("Error al abrir el archivo: {:?}", error));
    CommandError::FileOpenError(error.to_string())
}
