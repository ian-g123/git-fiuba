use std::io::Read;

use crate::command_errors::CommandError;

pub trait Pkt {
    fn to_pkt_format(&self) -> String;

    fn read_pkt_format(stream: &mut dyn Read) -> Result<Option<String>, CommandError>;
}

impl Pkt for String {
    fn to_pkt_format(&self) -> String {
        let input_len = self.len() + 4;
        let input_len_hex = format!("{:04x}", input_len);
        
        input_len_hex + self
    }

    /// lee una línea en formato pkt-line del stream
    fn read_pkt_format(stream: &mut dyn Read) -> Result<Option<String>, CommandError> {
        let size = read_pkt_size(stream)?;
        if size == 0 {
            return Ok(None);
        }
        let mut line_buffer = vec![0; size - 4];
        stream
            .read_exact(&mut line_buffer)
            .map_err(|e| CommandError::ErrorReadingPktVerbose(e.to_string()))?;
        let line = String::from_utf8(line_buffer)
            .map_err(|e| CommandError::ErrorReadingPktVerbose(e.to_string()))?;

        Ok(Some(line))
    }
}

fn read_pkt_size(socket: &mut dyn Read) -> Result<usize, CommandError> {
    let mut size_buffer = [0; 4];
    socket
        .read(&mut size_buffer)
        .map_err(|error| CommandError::ErrorReadingPktVerbose(error.to_string()))?;
    if size_buffer == [0; 4] {
        return Ok(0);
    }
    let from_utf8 = &String::from_utf8(size_buffer.to_vec())
        .map_err(|error| CommandError::ErrorReadingPktVerbose(error.to_string()))?;
    let size_vec = hex_string_to_u8_vec_2(from_utf8.as_str())?;

    let size: usize = u16::from_be_bytes(size_vec) as usize;
    Ok(size)
}

pub fn hex_string_to_u8_vec_2(hex_string: &str) -> Result<[u8; 2], CommandError> {
    if hex_string == "0000" || hex_string.is_empty() || hex_string.is_empty() {
        return Ok([0; 2]);
    }
    let mut result = [0; 2];
    let mut chars = hex_string.chars();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.next() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                i += 1;
            } else {
                return Err(CommandError::ErrorReadingPktVerbose(format!(
                    "Invalid hex string: '{}'",
                    hex_string
                )));
            }
        } else {
            break;
        }
    }

    Ok(result)
}
