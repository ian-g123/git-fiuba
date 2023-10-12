use super::error_flags::ErrorFlags;
use std::io::{Read, Write};

/// Característico de todos los comandos de git
pub trait Command {
    /// Instancia y corre el comando a partir del nombre y sus argumentos
    ///
    /// # Errors:
    ///
    /// - `ErrorFlags::CommandName` si el nombre del comando no es válido
    /// - `ErrorFlags::InvalidArgument` si se encuentra un argumento inválido
    /// - `ErrorFlags::WrongFlag` si se encuentra una flag inválida
    /// - `ErrorFlags::InvalidArguments` si se encuentra un argumento inválido
    /// - `ErrorFlags::FileNotFound` si no se encuentra el archivo
    /// - `ErrorFlags::FileReadError` si hay un error leyendo el archivo
    /// - `ErrorFlags::ObjectTypeError` si el tipo de objeto de -t no es válido
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags>;

    /// Método para diferenciar flags de valores
    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }
}
