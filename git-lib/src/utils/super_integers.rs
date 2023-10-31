use std::io::{Read, Write};

use crate::command_errors::CommandError;

/// Incluye funciones relacionadas a Integers.
pub trait SuperIntegers {
    /// Escribe el integer en el stream
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError>;
}

impl SuperIntegers for i64 {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let value_be = self.to_be_bytes();
        stream
            .write_all(&value_be)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

impl SuperIntegers for i32 {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let value_be = self.to_be_bytes();
        stream
            .write_all(&value_be)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

impl SuperIntegers for u32 {
    fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        let value_be = self.to_be_bytes();
        stream
            .write_all(&value_be)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        Ok(())
    }
}

/// Lee el integer del stream
pub fn read_i64_from(stream: &mut dyn Read) -> Result<i64, CommandError> {
    let mut value_be = [0; 8];
    stream
        .read_exact(&mut value_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let value = i64::from_be_bytes(value_be);
    Ok(value)
}

/// Lee el integer del stream
pub fn read_i32_from(stream: &mut dyn Read) -> Result<i32, CommandError> {
    let mut value_be = [0; 4];
    stream
        .read_exact(&mut value_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let value = i32::from_be_bytes(value_be);
    Ok(value)
}

/// Lee el integer del stream
pub fn read_u32_from(stream: &mut dyn Read) -> Result<u32, CommandError> {
    let mut parents_len_be = [0; 4];
    stream
        .read_exact(&mut parents_len_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let parents_len = u32::from_be_bytes(parents_len_be);
    Ok(parents_len)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_write_to_u32() {
        let value: u32 = 42;
        let mut buffer: Vec<u8> = Vec::new();

        value.write_to(&mut buffer).unwrap();

        let expected_bytes = 42u32.to_be_bytes();
        assert_eq!(buffer, expected_bytes.to_vec());
    }

    #[test]
    fn test_write_to_i32() {
        let value: i32 = 42;
        let mut buffer: Vec<u8> = Vec::new();

        value.write_to(&mut buffer).unwrap();

        let expected_bytes = 42i32.to_be_bytes();
        assert_eq!(buffer, expected_bytes.to_vec());
    }

    #[test]
    fn test_write_to_i64() {
        let value: i64 = 42;
        let mut buffer: Vec<u8> = Vec::new();

        value.write_to(&mut buffer).unwrap();

        let expected_bytes = 42i64.to_be_bytes();
        assert_eq!(buffer, expected_bytes.to_vec());
    }

    #[test]
    fn test_read_from_u32() {
        let value: u32 = 42;
        let mut bytes = 42u32.to_be_bytes();
        let mut stream = Cursor::new(&mut bytes);

        let result = read_u32_from(&mut stream).unwrap();

        assert_eq!(result, value);
    }

    #[test]
    fn test_read_from_i32() {
        let value: i32 = 42;
        let mut bytes = 42i32.to_be_bytes();
        let mut stream = Cursor::new(&mut bytes);

        let result = read_i32_from(&mut stream).unwrap();

        assert_eq!(result, value);
    }

    #[test]
    fn test_read_from_i64() {
        let value: i64 = 42;
        let mut bytes = 42i64.to_be_bytes();
        let mut stream = Cursor::new(&mut bytes);

        let result = read_i64_from(&mut stream).unwrap();

        assert_eq!(result, value);
    }
}
