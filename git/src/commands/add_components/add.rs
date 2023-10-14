use std::{
    collections::{HashMap, HashSet},
    error,
    fs::{self, DirEntry},
    io::{Read, Write},
};

use crate::{
    commands::{
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        hash_object_components::hash_object::HashObject,
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
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        for pathspec in &self.pathspecs {
            match fs::read_dir(pathspec) {
                Ok(mut it) => it.try_for_each(|entry| match entry {
                    Ok(entry) => {
                        self.run_for_entry(entry, output, logger)?;
                        Ok(())
                    }
                    Err(error) => return Err(CommandError::FileOpenError(error.to_string())),
                })?,
                Err(error) => return Err(CommandError::FileOpenError(error.to_string())),
            }
        }
        Ok(())
    }

    fn run_for_entry(
        &self,
        entry: DirEntry,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let path = entry.path();
        let path_str = path.to_str().unwrap().to_string();

        if path.is_dir() {
            logger.log(&format!("add {:?}", path_str));
            return Ok(());
        }

        let mut file = fs::File::open(path_str.clone()).unwrap();
        let mut content = Vec::<u8>::new();
        file.read(&mut content)
            .map_err(|error| CommandError::FileOpenError(error.to_string()))?;

        let hash_object = HashObject::new("blob".to_string(), vec![], true, false);
        hash_object.run_for_content(content)?;
        Ok(())
    }
}

// pub struct StagingArea {
//     stagin_area: Tree,
//     pointers: HashMap<String, String>,
// }

// pub struct Tree {
//     file_system: HashMap<String, SavedObjects>,
// }

// enum SavedObjects {
//     Blob(String),
//     Tree(Tree),
// }

// impl StagingArea {
//     pub fn add(&mut self, file_name: String) {
//         self.stagin_area.add_file(file_name);
//     }

//     pub fn new() -> Self {
//         StagingArea {
//             stagin_area: Tree::new(),
//             pointers: HashMap::new(),
//         }
//     }
// }

// impl Tree {
//     pub fn new() -> Self {
//         Tree {
//             file_system: HashMap::new(),
//         }
//     }

//     pub fn add_file(&mut self, file_name: String) {
//         let file = SavedObjects::Blob(file_name.clone());
//         self.file_system.insert(file_name, file);
//     }

//     pub fn add_tree(&mut self, folder_name: String) {
//         let tree = SavedObjects::Tree(Tree::new());
//         self.file_system.insert(folder_name, tree);
//     }

//     pub fn hash(&self) -> String {
//         "hash".to_string()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
}
