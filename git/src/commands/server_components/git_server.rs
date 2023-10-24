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

    pub fn explore_repository(
        &mut self,
        repository_path: &str,
        host: &str,
        logger: &mut Logger,
    ) -> Result<(String, Vec<(String, String)>), CommandError> {
        let line = format!(
            "git-upload-pack {}\0host={}\0\0version=1\0\n",
            repository_path, host
        );
        let mut lines = self.send(&line, logger)?;
        let first_line = lines.remove(0);
        if first_line != "version 1\n" {
            return Err(CommandError::ErrorReadingPkt);
        }
        let head_branch_line = lines.remove(0);
        let Some((head_branch, _other_words)) = head_branch_line.split_once(' ') else {
            return Err(CommandError::ErrorReadingPkt);
        };
        let mut refs = Vec::<(String, String)>::new();
        for line in lines {
            let (ref_name, sha1) = line.split_once(' ').ok_or(CommandError::ErrorReadingPkt)?;
            refs.push((ref_name.to_string(), sha1.to_string()));
        }
        Ok((head_branch.to_string(), refs))
    }
}

fn get_response(mut socket: &TcpStream, logger: &mut Logger) -> Result<Vec<String>, CommandError> {
    logger.log("Getting response");
    let mut lines = Vec::<String>::new();
    loop {
        match String::read_pkt_format(&mut socket)? {
            Some(line) => {
                lines.push(line);
            }
            None => break,
        }
    }
    Ok(lines)
}

fn read_pkt_size(socket: &mut dyn Read) -> Result<usize, CommandError> {
    let mut size_buffer = [0; 4];
    socket
        .read(&mut size_buffer)
        .map_err(|_| CommandError::ErrorReadingPkt)?;
    let from_utf8 =
        &String::from_utf8(size_buffer.to_vec()).map_err(|_| CommandError::ErrorReadingPkt)?;
    let size_vec = hex_string_to_u8_vec_2(from_utf8.as_str())?;
    let size: usize = u16::from_be_bytes(size_vec) as usize;
    Ok(size)
}

pub fn hex_string_to_u8_vec_2(hex_string: &str) -> Result<[u8; 2], CommandError> {
    let mut result = [0; 2];
    let mut chars = hex_string.chars();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.next() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                i += 1;
            } else {
                return Err(CommandError::ErrorReadingPkt);
            }
        } else {
            break;
        }
    }

    Ok(result)
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
        let size = read_pkt_size(stream)?;
        if size == 0 {
            return Ok(None);
        }
        let mut line_buffer = vec![0; size - 4];
        stream
            .read_exact(&mut line_buffer)
            .map_err(|_| CommandError::ErrorReadingPkt)?;
        let line = String::from_utf8(line_buffer).map_err(|_| CommandError::ErrorReadingPkt)?;
        Ok(Some(line))
    }
}
