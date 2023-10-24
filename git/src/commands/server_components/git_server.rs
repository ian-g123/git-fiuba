use crate::commands::{command_errors::CommandError, objects::aux::*};
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

    pub fn send(&mut self, line: &str) -> Result<Vec<String>, CommandError> {
        let line = line.to_string().to_pkt_format();
        self.socket
            .write_all(line.as_bytes())
            .map_err(|_| CommandError::SendingMessage)?;
        let lines = get_response(&self.socket)?;
        Ok(lines)
    }
}

fn get_response(mut socket: &TcpStream) -> Result<Vec<String>, CommandError> {
    let mut lines = Vec::<String>::new();
    loop {
        let line = String::read_pkt_format(&mut socket)?;
        if line == "0000" {
            break;
        }
        lines.push(line);
    }
    Ok(lines)
}

trait Pkt {
    fn to_pkt_format(&self) -> String;

    fn read_pkt_format(stream: &mut dyn Read) -> Result<String, CommandError>;
}

impl Pkt for String {
    fn to_pkt_format(&self) -> String {
        let input_len = self.len() + 4;
        let input_len_hex = format!("{:04x}", input_len);
        let output = input_len_hex + self;
        output
    }

    fn read_pkt_format(stream: &mut dyn Read) -> Result<String, CommandError> {
        let mut len_hex = [0; 4];
        stream
            .read_exact(&mut len_hex)
            .map_err(|_| CommandError::ErrorReadingPkt)?;
        let len_str =
            String::from_utf8(len_hex.to_vec()).map_err(|_| CommandError::ErrorReadingPkt)?;
        let len_vec = len_str.cast_hex_to_u8_vec()?;
        let mut len_be = [0; 4];
        len_be.copy_from_slice(&len_vec);

        let len = u32::from_be_bytes(len_be) as usize;
        let mut buf = vec![0; len];
        stream
            .read_exact(&mut buf)
            .map_err(|_| CommandError::ErrorReadingPkt)?;
        let output = String::from_utf8(buf).map_err(|_| CommandError::ErrorReadingPkt)?;
        Ok(output)
    }
}
