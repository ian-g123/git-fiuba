use libflate::deflate::{Decoder, Encoder};
use std::{
    io::{Read, Write},
    process::Command,
};

use super::command_errors::CommandError;

pub fn compress(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut encoder = Encoder::new(Vec::new());
    encoder
        .write_all(data)
        .map_err(|error| CommandError::CompressionError);
    encoder
        .finish()
        .into_result()
        .map_err(|_| CommandError::CompressionError)
}

pub fn extract(data: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut decoder = Decoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .map_err(|error| CommandError::CompressionError);
    Ok(decompressed_data)
}
