use std::{
    fmt, fs,
    io::{Read, Write},
    os::unix::prelude::PermissionsExt,
};

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

    // Cambios que hizo Ian
    pub fn get_id_mode(&self) -> u32 {
        match self {
            Mode::RegularFile => 100644,
            Mode::ExecutableFile => 100755,
            Mode::SymbolicLink => 120000,
            Mode::Submodule => 160000,
            Mode::Tree => 040000,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<Self, CommandError> {
        let mut buf = [0; 6];
        stream
            .read_exact(&mut buf)
            .map_err(|_| CommandError::InvalidMode)?;
        let mode = std::str::from_utf8(&buf).map_err(|_| CommandError::InvalidMode)?;
        Ok(Self::read_from_string(mode)?)
    }

    pub fn read_from_string(mode: &str) -> Result<Self, CommandError> {
        match mode {
            "100644" => Ok(Mode::RegularFile),
            "100755" => Ok(Mode::ExecutableFile),
            "120000" => Ok(Mode::SymbolicLink),
            "160000" => Ok(Mode::Submodule),
            "040000" => Ok(Mode::Tree),
            _ => Err(CommandError::InvalidMode),
        }
    }

    pub(crate) fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let mode_str = match self {
            Mode::RegularFile => "100644",
            Mode::ExecutableFile => "100755",
            Mode::SymbolicLink => "120000",
            Mode::Submodule => "160000",
            Mode::Tree => "040000",
        };
        stream.write(mode_str.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(format!("Error al escribir el mode: {}", error))
        })?;
        Ok(())
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode_str = match self {
            Mode::RegularFile => "100644",
            Mode::ExecutableFile => "100755",
            Mode::SymbolicLink => "120000",
            Mode::Submodule => "160000",
            Mode::Tree => "040000",
        };
        write!(f, "{}", mode_str)
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
    fn get_mode_regular_file() {
        let path = String::from("tests/data/mode/blob.txt");
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
        let path = String::from("tests/data/mode/folder");
        fs::create_dir(&path).unwrap();
        assert!(matches!(Mode::get_mode(path), Ok(Mode::Tree)))
    }

    /// Si el <path> pasado a get_mode() corresponde a un link simbólico, la función devuelve
    /// Mode::SymbolicLink.
    #[test]
    #[ignore = "No estamos seguro de como funcionan los symlink"]
    fn get_mode_sym_link() {
        let path = String::from("tests/data/mode/link");
        assert!(matches!(Mode::get_mode(path), Ok(Mode::SymbolicLink)))
    }
}
