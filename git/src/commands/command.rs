use crate::logger::Logger;

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
        logger: &mut Logger,
    ) -> Result<(), ErrorFlags>;

    /// Método para diferenciar flags de valores
    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }

    /// Configura el comando a partir de los argumentos
    fn config(&mut self, args: &[String]) -> Result<(), ErrorFlags> {
        let mut i = 0;
        while i < args.len() {
            i = self.add_config(i, args)?;
        }
        Ok(())
    }

    /// Configura el valor en i a partir de los argumentos
    fn add_config(&mut self, i: usize, args: &[String]) -> Result<usize, ErrorFlags> {
        for f in self.config_adders().iter() {
            match f(self, i, args) {
                Ok(i) => return Ok(i),
                Err(ErrorFlags::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ErrorFlags::InvalidArguments)
    }

    /// Devuelve un vector de funciones que parsean flags
    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, ErrorFlags>>;
}
