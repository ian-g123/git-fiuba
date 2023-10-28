use libflate::deflate::{Decoder, Encoder};
use std::io::{Read, Write};

use super::command_errors::CommandError;

/// Comprime un vector de bytes
pub fn compress(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    // compress data with zlib from flate2 crate
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).unwrap();
    let compressed_data = encoder.finish().unwrap();
    Ok(compressed_data)
}

/// Descomprime un vector de bytes
pub fn extract(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data).unwrap();
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
