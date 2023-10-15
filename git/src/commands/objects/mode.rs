use std::{fs, os::unix::prelude::PermissionsExt};

use crate::commands::command_errors::CommandError;

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    RegularFile = 100644,
    ExecutableFile = 100755,
    SymbolicLink = 120000,
    Submodule = 160000,
    Tree = 040000,
}

impl Mode {
    /// Devuelve el Mode del path recibido. Si éste no existe, devuelve Error.
    pub fn get_mode(path: String) -> Result<Mode, CommandError> {
        let mode: Mode;
        let Ok(metadata) = fs::metadata(path.clone()) else {
            return Err(CommandError::FileNotFound(path));
        };

        let permissions_mode = metadata.permissions().mode();

        if metadata.is_dir() {
            mode = Mode::Tree;
        } else if metadata.is_symlink() {
            mode = Mode::SymbolicLink;
        } else if (permissions_mode & 0o111) != 0 {
            mode = Mode::ExecutableFile;
        } else {
            mode = Mode::RegularFile;
        }

        Ok(mode)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Si el <path> pasado a get_mode() no existe, la función devuelve error.
    #[test]
    fn get_mode_fails() {
        let path = String::from("no_existe");

        assert!(matches!(
            Mode::get_mode(path),
            Err(CommandError::FileNotFound(path))
        ));
    }

    /// Si el `<path>` pasado a `get_mode()` corresponde a un archivo regular, la función devuelve
    /// Mode::RegularFile.
    #[test]
    #[ignore]
    fn get_mode_regular_file() {
        let path = String::from("../.gitignore");
        assert!(matches!(Mode::get_mode(path), Ok(Mode::RegularFile)))
    }

    /* /// Si el <path> pasado a get_mode() corresponde a un archivo ejecutable, la función devuelve
    /// Mode::ExecutableFile.
    #[test]
    fn get_mode_exe_file(){
        let path = String::from("/usr/bin");
        assert!(matches!(Mode::get_mode(path), Ok(Mode::RegularFile)))
    } */

    /// Si el <path> pasado a get_mode() corresponde a un directorio, la función devuelve
    /// Mode::Tree.
    #[test]
    #[ignore]
    fn get_mode_tree() {
        let path = String::from("../git");
        assert!(matches!(Mode::get_mode(path), Ok(Mode::Tree)))
    }
}
