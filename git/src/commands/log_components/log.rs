use std::{
    fs::File,
    io::{self, Cursor, Read, Write},
    str,
};

use crate::{
    commands::{
        cat_file_components::cat_file::CatFile,
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        file_compressor,
        objects::commit_object::{self, read_from, read_from_for_log},
    },
    logger::Logger,
};

/// Commando init
pub struct Log {
    all: bool,
}

impl Command for Log {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
        _logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "log" {
            return Err(CommandError::Name);
        }

        let instance = Self::new(args)?;

        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![Self::add_all_config]
    }
}

impl Log {
    fn new(args: &[String]) -> Result<Self, CommandError> {
        let mut log = Self::new_default();
        log.config(args)?;
        Ok(log)
    }

    fn new_default() -> Self {
        Self { all: false }
    }

    fn add_all_config(log: &mut Log, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--all" {
            return Err(CommandError::WrongFlag);
        }
        log.all = true;
        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let path_to_actual_branch = get_path_to_actual_branch()?;
        let mut file = File::open(path_to_actual_branch).map_err(|_| {
            CommandError::FileNotFound("No se pudo abrir .git/refs/heads en log".to_string())
        })?;

        let mut commit_hash = String::new();
        file.read_to_string(&mut commit_hash).map_err(|_| {
            CommandError::FileReadError("No se pudo leer .git/refs/heads en log".to_string())
        })?;

        let mut logger = Logger::new_dummy();
        let deflated_file = file_compressor::extract(&commit_hash.as_bytes())?;
        let mut deflated_file_reader = Cursor::new(deflated_file);
        let commit_object = read_from_for_log(&mut deflated_file_reader, &mut logger)?;

        let parents = commit_object.get_parents();

        // imprimimos el padre en el output
        writeln!(output, "{}", commit_object.parent)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

// fn get_cat_file(hash: &str) -> Result<String, CommandError> {
//     let args = vec!["-p".to_string(), hash.to_string()];
//     let mut output_cat = Vec::new();
//     let mut logger = Logger::new_dummy();
//     CatFile::run_from(
//         "cat-file",
//         &args,
//         &mut io::stdin(),
//         &mut output_cat,
//         &mut logger,
//     )?;
//     let cat_file = str::from_utf8(&output_cat).map_err(|_| {
//         CommandError::FileReadError("No se pudo leer el output de cat-file en log".to_string())
//     })?;
//     Ok(cat_file.to_string())
// }

// fn get_parent(string: &String) -> Result<String, CommandError> {
//     let mut lines = string.lines();
//     let mut parents = Vec::new();
//     lines.next();
//     while let Some(line) = lines.next() {
//         if !line.starts_with("parent") {
//             break;
//         }
//         let parent = line.to_string();
//         parents.push(parent);
//     }
//     let parent = parent.split_once(' ').ok_or(CommandError::FileReadError(
//         "No se pudo leer el output de cat-file en log".to_string(),
//     ))?;
//     Ok(parent.1.to_string())
// }

fn get_path_to_actual_branch() -> Result<String, CommandError> {
    let mut file = File::open(".git/HEAD")
        .map_err(|_| CommandError::FileNotFound("No se pudo abrir .git/HEAD en log".to_string()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|_| CommandError::FileReadError("No se pudo leer .git/HEAD en log".to_string()))?;
    let Some((_, path_to_branch)) = contents.split_once(' ') else {
        return Err(CommandError::FileNotFound(
            "No se pudo abrir .git/HEAD en log".to_string(),
        ));
    };
    let branch = format!(".git/{}", path_to_branch);
    Ok(branch)
}
