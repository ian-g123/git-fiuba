use crate::command_errors::CommandError;
use crate::objects::{blob::Blob, tree::Tree};
use crate::utils::aux::get_name;
use std::{
    fs,
    path::{Path, PathBuf},
};

// build working tree
pub fn build_working_tree(working_dir: &str) -> Result<Tree, CommandError> {
    let mut tree = Tree::new("".to_string());
    build_working_tree_aux(working_dir, &mut tree)?;
    Ok(tree)
}

fn build_working_tree_aux(path_name: &str, tree: &mut Tree) -> Result<(), CommandError> {
    let path = if path_name.is_empty() {
        Path::new("./")
    } else {
        Path::new(path_name)
    };

    let Ok(entries) = fs::read_dir(path.clone()) else {
        return Err(CommandError::DirNotFound(path_name.to_owned()));
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return Err(CommandError::DirNotFound(path_name.to_owned()));
        };
        let entry_path = entry.path();
        let full_path = &get_path_name(entry_path.clone())?;
        // warning: stripping a prefix manually
        //   --> git-lib/src/changes_controller_components/working_tree.rs:33:13
        //   |
        // 33 |             &full_path[2..]
        //   |             ^^^^^^^^^^^^^^^
        //   |
        // note: the prefix was tested here
        //  --> git-lib/src/changes_controller_components/working_tree.rs:32:20
        //   |
        // 32 |         let path = if full_path.starts_with("./") {
        //   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        //   = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#manual_strip
        //   = note: `#[warn(clippy::manual_strip)]` on by default
        // help: try using the `strip_prefix` method
        //   |
        // 32 ~         let path = if let Some(<stripped>) = full_path.strip_prefix("./") {
        // 33 ~             <stripped>
        let path = if full_path.starts_with("./") {
            &full_path[2..]
        } else {
            full_path
        };
        if full_path.contains(".git") {
            continue;
        }
        if entry_path.is_dir() {
            let mut new_tree = Tree::new(path.to_owned());
            build_working_tree_aux(full_path, &mut new_tree)?;
            tree.add_object(get_name(path)?, Box::new(new_tree))?;
        } else {
            let content = fs::read(entry_path.clone())
                .map_err(|_| CommandError::FileNotFound(full_path.to_owned()))?;
            let blob = Blob::new_from_content_and_path(content, path)?;
            tree.add_object(get_name(path)?, Box::new(blob))?;
        }
    }
    Ok(())
}

/// Devuelve el nombre de un archivo o directorio dado un PathBuf.
pub fn get_path_name(path: PathBuf) -> Result<String, CommandError> {
    let Some(path_name) = path.to_str() else {
        return Err(CommandError::DirNotFound("".to_string())); //cambiar
    };
    Ok(path_name.to_string())
}
