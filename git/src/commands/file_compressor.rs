use libflate::deflate::{Decoder, Encoder};
use std::io::{Read, Write};

use super::command_errors::CommandError;

/// Comprime un vector de bytes
pub fn compress(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut encoder = Encoder::new(Vec::new());
    let _ = encoder
        .write_all(data)
        .map_err(|_error| CommandError::CompressionError);
    encoder
        .finish()
        .into_result()
        .map_err(|_| CommandError::CompressionError)
}

/// Descomprime un vector de bytes
pub fn extract(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut decoder = Decoder::new(data);
    let mut decompressed_data = Vec::new();
    let _ = decoder
        .read_to_end(&mut decompressed_data)
        .map_err(|_error| CommandError::CompressionError);
    Ok(decompressed_data)
}

#[cfg(test)]
mod test_compress_and_extract {
    use super::*;

    #[test]
    fn test_compress_and_extract() {
        let data = b"Hello, world!";
        let compressed_data = compress(data).unwrap();
        let extracted_data = extract(&compressed_data).unwrap();
        assert_eq!(extracted_data, b"Hello, world!");
    }
}
