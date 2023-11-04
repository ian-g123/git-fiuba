use crate::command_errors::CommandError;
use crate::objects::{blob::Blob, tree::Tree};
use crate::utils::aux::get_name;
use std::{
    fs,
    path::{Path, PathBuf},
};

// build working tree
pub fn build_working_tree() -> Result<Tree, CommandError> {
    let path = "./";
    let mut tree = Tree::new("".to_string());
    build_working_tree_aux(path, &mut tree)?;
    Ok(tree)
}

fn build_working_tree_aux(path_name: &str, tree: &mut Tree) -> Result<(), CommandError> {
    let path = Path::new(path_name);

    let Ok(entries) = fs::read_dir(path.clone()) else {
        return Err(CommandError::DirNotFound(path_name.to_owned()));
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return Err(CommandError::DirNotFound(path_name.to_owned()));
        };
        let entry_path = entry.path();
        let full_path = &get_path_name(entry_path.clone())?;
        let path = &full_path[2..];
        if full_path.contains("./.git") {
            continue;
        }
        if entry_path.is_dir() {
            let mut new_tree = Tree::new(path.to_owned());
            build_working_tree_aux(&full_path, &mut new_tree)?;
            tree.add_object(get_name(path)?, Box::new(new_tree));
        } else {
            let blob = Blob::new_from_path(path.to_string())?;
            tree.add_object(get_name(path)?, Box::new(blob));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print_working_tree() {
        let wt = build_working_tree().unwrap();
        validate_tree(wt);
    }

    fn validate_tree(tree: Tree) {
        for (name, object) in tree.get_objects().iter_mut() {
            if let Some(new_tree) = object.as_tree() {
                validate_tree(new_tree.clone())
            } else {
                let path = PathBuf::from(name);
                assert!(path.exists(), "File name: {}", name);
            }
        }
    }
}
