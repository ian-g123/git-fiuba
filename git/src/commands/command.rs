use git_lib::command_errors::CommandError;
use git_lib::logger::Logger;
use std::io::{Read, Write};

/// Función que agrega un valor a partir de los argumentos
pub type ConfigAdderFunction<T> = Vec<fn(&mut T, usize, &[String]) -> Result<usize, CommandError>>;

/// Característico de todos los comandos de git
pub trait Command {
    /// Instancia y corre el comando a partir del nombre y sus argumentos
    ///
    /// # Errors:
    ///
    /// - `CommandError::CommandName` si el nombre del comando no es válido
    /// - `CommandError::InvalidArgument` si se encuentra un argumento inválido
    /// - `CommandError::WrongFlag` si se encuentra una flag inválida
    /// - `CommandError::InvalidArguments` si se encuentra un argumento inválido
    /// - `CommandError::FileNotFound` si no se encuentra el archivo
    /// - `CommandError::FileReadError` si hay un error leyendo el archivo
    /// - `CommandError::ObjectTypeError` si el tipo de objeto de -t no es válido
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError>;

    /// Método para diferenciar flags de valores
    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }

    /// Configura el comando a partir de los argumentos
    fn config(&mut self, args: &[String]) -> Result<(), CommandError> {
        let mut i = 0;
        while i < args.len() {
            i = self.add_config(i, args)?;
        }
        Ok(())
    }

    /// Configura el valor en i a partir de los argumentos
    fn add_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        for f in self.config_adders().iter() {
            match f(self, i, args) {
                Ok(i) => return Ok(i),
                Err(CommandError::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(CommandError::InvalidArguments)
    }

    /// Devuelve un vector de funciones que parsean flags
    fn config_adders(&self) -> ConfigAdderFunction<Self>;
}
