use std::fmt;

#[derive(Debug)]
pub enum ErrorFlags {
    CommandName,
    WrongFlag,
    InvalidArguments,
    FileNotFound,
    FileReadError,
    ObjectTypeError,
}

impl fmt::Display for ErrorFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorFlags::CommandName => write!(f, "El nombre del comando no es válido"),
            ErrorFlags::WrongFlag => write!(f, "La flag no es válida"),
            ErrorFlags::InvalidArguments => write!(f, "Argumentos inválidos"),
            ErrorFlags::FileNotFound => write!(f, "Archivo no encontrado"),
            ErrorFlags::FileReadError => write!(f, "Error leyendo archivo"),
            ErrorFlags::ObjectTypeError => write!(f, "Tipo de objeto no válido"),
        }
    }
}
