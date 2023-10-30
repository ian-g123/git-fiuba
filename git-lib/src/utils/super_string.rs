use std::io::{Read, Write};

use crate::command_errors::CommandError;

/// Incluye funciones relacionadas a Strings.
pub trait SuperStrings {
    /// Escribe un string en un stram.
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError>;

    /// Realiza un casteo de hexadecimal a Vec<u8>.
    fn cast_hex_to_u8_vec(&self) -> Result<[u8; 20], CommandError>;
}

impl SuperStrings for String {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let len_be = (self.len() as u32).to_be_bytes();
        stream
            .write_all(&len_be)
            .map_err(|error| CommandError::FileWriteError(error.to_string() + "AAAA"))?;

        stream.write_all(self.as_bytes()).map_err(|error| {
            CommandError::FileWriteError(format!("Error escribiendo en el stream: {}", error))
        })?;
        Ok(())
    }

    fn cast_hex_to_u8_vec(&self) -> Result<[u8; 20], CommandError> {
        let mut result = [0; 20];
        let mut chars = self.chars();

        let mut i = 0;
        while let Some(c1) = chars.next() {
            if let Some(c2) = chars.next() {
                if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                    result[i] = (n1 * 16 + n2) as u8;
                    i += 1;
                } else {
                    return Err(CommandError::CastingError);
                }
            } else {
                break;
            }
        }

        Ok(result)
    }
}

/// Lee de un stream y crea un string.
pub fn read_string_from(stream: &mut dyn Read) -> Result<String, CommandError> {
    let mut len_be = [0; 4];
    stream
        .read_exact(&mut len_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let len = u32::from_be_bytes(len_be) as usize;

    let mut content = vec![0; len];
    stream
        .read_exact(&mut content)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;

    let result = String::from_utf8(content).map_err(|error| {
        CommandError::FileReadError(format!("Error leyendo el stream: {}", error))
    })?;
    Ok(result)
}

/// Realiza un casteo de Vec<u8> a string hexadecimal.
pub fn u8_vec_to_hex_string(u8_vec: &[u8]) -> String {
    let hex_string = u8_vec
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("");

    hex_string
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Cursor, Seek, SeekFrom};

    /// Prueba que un SuperString se pueda leer y escribir.
    #[test]
    fn test_read_and_write() {
        let string_expected = "my super string".to_string();
        let buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(buf);
        string_expected.write_to(&mut stream).unwrap();
        stream.seek(SeekFrom::Start(0)).unwrap();
        let result = read_string_from(&mut stream).unwrap();
        assert_eq!(result, string_expected)
    }

    /// Prueba que se pueda castear de hexadecimal a Vec<u8>.
    #[test]
    fn test_cast_hex_to_u8_vec() {
        let hex_string = "1a2b3c".to_string();
        let expected_result: [u8; 20] = [
            0x1a, 0x2b, 0x3c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let result = hex_string.cast_hex_to_u8_vec().unwrap();

        assert_eq!(result, expected_result);
    }

    /// Prueba que se pueda castear de Vec<u8> a hexadecimal.
    #[test]
    fn test_u8_vec_to_hex_string() {
        let input = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let expected = "48656c6c6f";

        let result = u8_vec_to_hex_string(&input);
        assert_eq!(result, expected);
    }
}
