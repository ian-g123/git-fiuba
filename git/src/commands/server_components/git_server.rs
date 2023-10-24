use crate::{
    commands::{command_errors::CommandError, objects::aux::*},
    logger::Logger,
};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

pub struct GitServer {
    socket: TcpStream,
}

impl GitServer {
    pub fn connect_to(address: &str) -> Result<GitServer, CommandError> {
        let socket = TcpStream::connect(address).map_err(|error| {
            CommandError::Connection(format!(
                "No se pudo conectar al servidor en la dirección {}",
                error
            ))
        })?;
        Ok(GitServer { socket })
    }

    pub fn send(&mut self, line: &str, logger: &mut Logger) -> Result<Vec<String>, CommandError> {
        let line = line.to_string().to_pkt_format();
        logger.log(&format!("Sending: {}", line));
        self.socket
            .write_all(line.as_bytes())
            .map_err(|_| CommandError::SendingMessage)?;
        logger.log(&format!("Sent: {}", line));
        let lines = get_response(&self.socket, logger)?;
        Ok(lines)
    }
}

fn get_response(mut socket: &TcpStream, logger: &mut Logger) -> Result<Vec<String>, CommandError> {
    logger.log("Getting response");
    let mut lines = Vec::<String>::new();
    loop {
        match String::read_pkt_format(&mut socket)? {
            Some(line) => {
                logger.log(&format!("Received: {}", line));
                lines.push(line);
            }
            None => break,
        }
    }
    Ok(lines)
}

fn read_pkt_size(mut socket: &mut dyn Read) -> usize {
    let mut size_buffer = [0; 4];
    socket.read(&mut size_buffer).unwrap();
    let size_vec =
        hex_string_to_u8_vec_2(String::from_utf8(size_buffer.to_vec()).unwrap().as_str());
    let size: usize = u16::from_be_bytes(size_vec) as usize;
    size
}

pub fn hex_string_to_u8_vec_2(hex_string: &str) -> [u8; 2] {
    let mut result = [0; 2];
    let mut chars = hex_string.chars();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.next() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                i += 1;
            } else {
                panic!("Invalid hex string");
            }
        } else {
            break;
        }
    }

    result
}

trait Pkt {
    fn to_pkt_format(&self) -> String;

    fn read_pkt_format(stream: &mut dyn Read) -> Result<Option<String>, CommandError>;
}

impl Pkt for String {
    fn to_pkt_format(&self) -> String {
        let input_len = self.len() + 3;
        let input_len_hex = format!("{:04x}", input_len);
        let output = input_len_hex + self;
        output
    }

    fn read_pkt_format(stream: &mut dyn Read) -> Result<Option<String>, CommandError> {
        // let mut len_hex = [0; 4];
        // stream
        //     .read_exact(&mut len_hex)
        //     .map_err(|_| CommandError::ErrorReadingPkt)?;
        // let len_str =
        //     String::from_utf8(len_hex.to_vec()).map_err(|_| CommandError::ErrorReadingPkt)?;
        // let len_vec = len_str.cast_hex_to_u8_vec()?;
        // let mut len_be = [0; 2];
        // len_be.copy_from_slice(&len_vec);

        // let len = u16::from_be_bytes(len_be) as usize;
        // println!("len: {}", len);
        // if len == 0 {
        //     return Ok("".to_string());
        // }
        // let mut buf = vec![0; len];
        // stream
        //     .read_exact(&mut buf)
        //     .map_err(|_| CommandError::ErrorReadingPkt)?;
        // let output = String::from_utf8(buf.clone()).map_err(|_| CommandError::ErrorReadingPkt)?;
        // println!("buf: {:?}", buf);
        // println!("buf.len(): {}", buf.len());
        // println!("output: {}", output);
        // Ok(output)

        let size = read_pkt_size(stream);
        if size == 0 {
            return Ok(None);
        }
        let mut line_buffer = vec![0; size - 4];
        stream.read_exact(&mut line_buffer).unwrap();
        let line = String::from_utf8(line_buffer).unwrap();
        Ok(Some(line))
    }
}
