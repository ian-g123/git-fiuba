use std::{
    env,
    fs::{self},
    io::{Read, Write},
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

/// Se obtiene el nombre de los archivos y directorios del workspace\
/// Útil para cuando tengamos que hacer la interfaz de Stage Area
// fn get_files_names(path: &str) -> HashSet<String> {
//     // TODO: remover los unwrap
//     let mut files_names = HashSet::new();

//     for entry in WalkDir::new(path) {
//         let entry = entry.unwrap();

//         let file_name = entry.file_name().to_str().unwrap().to_string();
//         let directory_name = entry.path().to_str().unwrap().to_string();

//         files_names.insert(file_name);
//     }
//     files_names
// }

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
        logger.log(&format!("run: {:?}", &path));
        let path = Path::new(path);
        match fs::read_dir(path.to_str().unwrap()) {
            Err(error) => {
                if path.is_file() {
                    let Some(path_str) = path.to_str() else {
                        return Err(CommandError::FileOpenError(
                            "No se pudo convertir el path a str".to_string(),
                        ));
                    };
                    run_for_file(path_str, logger)?;
                    logger.log(&format!("Add: {:?}", path_str));
                    return Ok(());
                }
                logger.log(&format!(
                    "Error en read_dir: {:?} desde {:?}",
                    error,
                    env::current_dir()
                ));
                return Err(CommandError::FileOpenError(error.to_string()));
            }
            Ok(read_dir) => {
                for entry in read_dir {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let path_str = path.to_str().unwrap();
                            if path_str == "./.git" {
                                continue;
                            }
                            logger.log(&format!("entry: {:?}", path_str));
                            self.run_for_path(path_str, _output, logger)?;
                        }
                        Err(error) => {
                            logger.log(&format!("Error en entry: {:?}", error));
                            return Err(CommandError::FileOpenError(error.to_string()));
                        }
                    }
                }
            }
        };
        Ok(())
    }
}

fn run_for_file(path: &str, logger: &mut Logger) -> Result<(), CommandError> {
    let mut file = fs::File::open(path).unwrap();
    let mut content = Vec::<u8>::new();
    file.read_to_end(&mut content).unwrap();
    let hash_object = HashObject::new("blob".to_string(), vec![], true, false);
    let (hash_hex, _) = hash_object.run_for_content(content)?;
    match StagingArea::open() {
        Ok(mut staging_area) => {
            staging_area.add(path, &hash_hex);
            staging_area.save()?;
        }
        Err(error) => {
            logger.log(&format!("Error al abrir el staging area: {:?}", error));
            return Err(CommandError::FailToOpenSatginArea(error.to_string()));
        }
    }
    Ok(())
}
