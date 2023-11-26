use std::{
    fmt,
    io::{Read, Write},
};

use crate::command_errors::CommandError;

use crate::utils::aux::read_string_until;

/// Un Author realiza cambios en el repositorio y/o los commitea. Su información incluye nombre,
/// apellido e email.
#[derive(Clone, PartialEq, Debug)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    /// Dado un nombre y un email, crea un Author.
    pub fn new(name: &str, email: &str) -> Self {
        Author {
            name: name.to_string(),
            email: email.to_string(),
        }
    }

    /// Crea un Author a partir de su información (nombre, apellido, email).
    pub fn from_strings(strings: &mut Vec<&str>) -> Result<Self, CommandError> {
        let email_string = strings.pop();
        let email = match email_string {
            Some(email) => email[1..email.len() - 1].to_string(),
            None => return Err(CommandError::InvalidAuthor),
        };

        let name = strings.join(" ");
        Ok(Author { name, email })
    }

    /// Guarda la información del Author en el stream pasado.
    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        write!(stream, "{} <{}> ", self.name, self.email)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        // self.name.write_to(stream)?;
        // self.email.write_to(stream)?;
        Ok(())
    }

    /// Lee información del stream pasado y la usa para crear un Author.
    pub fn read_from(stream: &mut dyn Read) -> Result<Self, CommandError> {
        let name_part = &read_string_until(stream, '<')?;
        let name = name_part.trim().to_string();
        let mut email = read_string_until(stream, ' ')?.trim().to_string();
        email.pop();
        // let name = read_string_from(stream)?;
        // let email = read_string_from(stream)?;
        Ok(Self { name, email })
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Seek, SeekFrom};

    use super::*;

    /// Prueba que se pueda crear un Author a partir de un vector de strings.
    #[test]
    fn create_author_from_string() {
        let mut string = ["Juan", "Perez", "<jperez@fi.uba.ar>"].to_vec();
        let author_expected = Author::new("Juan Perez", "jperez@fi.uba.ar");
        let result = Author::from_strings(&mut string).unwrap();
        assert_eq!(result, author_expected)
    }

    /// Prueba que un Author se pueda leer y escribir.
    #[test]
    fn test_read_and_write() {
        let author_expected = Author::new("Juan Perez", "jperez@fi.uba.ar");
        let buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(buf);
        author_expected.write_to(&mut stream).unwrap();
        stream.seek(SeekFrom::Start(0)).unwrap();
        let result = Author::read_from(&mut stream).unwrap();
        assert_eq!(result, author_expected)
    }
}
