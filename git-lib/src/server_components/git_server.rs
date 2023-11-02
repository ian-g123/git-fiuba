use crate::{command_errors::CommandError, logger::Logger};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
};

use super::{packfile_object_type::PackfileObjectType, reader::TcpStreamBuffedReader};

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

    /// envía un mensaje al servidor y devuelve la respuesta
    fn send(&mut self, line: &str) -> Result<Vec<String>, CommandError> {
        let line = line.to_string().to_pkt_format();
        self.write_string_to_socket(&line)?;
        let lines = get_response(&self.socket)?;
        Ok(lines)
    }

    /// explora el repositorio remoto y devuelve el hash del commit del branch head
    /// y un hashmap con: hash del commit -> nombre de la referencia
    pub fn explore_repository(
        &mut self,
        repository_path: &str,
        host: &str,
    ) -> Result<(String, HashMap<String, String>), CommandError> {
        let line = format!(
            "git-upload-pack {}\0host={}\0\0version=1\0\n",
            repository_path, host
        );
        let mut lines = self.send(&line)?;
        let first_line = lines.remove(0);
        if first_line != "version 1\n" {
            return Err(CommandError::ErrorReadingPkt);
        }
        let head_branch_line = lines.remove(0);
        let Some((head_branch_commit, _)) = head_branch_line.split_once(' ') else {
            return Err(CommandError::ErrorReadingPkt);
        };
        let mut refs = HashMap::<String, String>::new();
        for line in lines {
            // logger.log(&format!("Line: {}", line));
            let (hash, ref_name) = line
                .split_once(' ')
                .ok_or(CommandError::ErrorReadingPkt)
                .map(|(sha1, ref_name)| (sha1.trim().to_string(), ref_name.trim().to_string()))?;
            refs.insert(hash, ref_name);
        }
        Ok((head_branch_commit.to_string(), refs))
    }

    /// explora el repositorio remoto y devuelve el hash del commit del branch head
    /// y un hashmap con: hash del commit -> nombre de la referencia
    pub fn explore_repository_receive(
        &mut self,
        repository_path: &str,
        host: &str,
    ) -> Result<HashMap<String, String>, CommandError> {
        let line = format!(
            "git-receive-pack {}\0host={}\0\0version=1\0\n",
            repository_path, host
        );

        let lines = self.send(&line)?;
        let mut refs = HashMap::<String, String>::new();

        for line in lines {
            let (hash, ref_name) = line
                .split_once(' ')
                .ok_or(CommandError::ErrorReadingPkt)
                .map(|(hash, ref_name)| (hash.trim().to_string(), ref_name.trim().to_string()))?;
            refs.insert(ref_name, hash);
        }
        Ok(refs)
    }

    /// envía un mensaje al servidor para que envíe los objetos del repositorio
    /// y devuelve un vector con tuplas que contienen:\
    /// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
    pub fn fetch_objects(
        &mut self,
        wants_commits: Vec<String>,
        haves_commits: Vec<String>,
        logger: &mut Logger,
    ) -> Result<Vec<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
        logger.log("fetch_objects");
        let mut lines = Vec::<String>::new();
        for want_commit in wants_commits {
            let line = format!("want {}\n", want_commit);
            logger.log(&format!("Sending: {}", line));
            self.write_in_tpk_to_socket(&line)?;
        }
        self.write_string_to_socket("0000")?;
        if !haves_commits.is_empty() {
            for have in haves_commits {
                let line = format!("have {}\n", have);
                logger.log(&format!("Sending:: {}", line));
                self.write_in_tpk_to_socket(&line)?;
            }
            self.write_string_to_socket("0000")?;
        }
        self.write_in_tpk_to_socket("done\n")?;
        logger.log("reading objects");

        match String::read_pkt_format(&mut self.socket)? {
            Some(line) => {
                logger.log(&format!("pushing: {:?}", line));
                lines.push(line);
            }
            None => return Err(CommandError::ErrorReadingPkt),
        }
        Ok(self.read_objects()?)
    }

    fn write_string_to_socket(&mut self, line: &str) -> Result<(), CommandError> {
        // self.write_to_socket(line.as_bytes());
        let message = line.as_bytes();
        self.socket
            .write_all(message)
            .map_err(|error| CommandError::SendingMessage(error.to_string()))?;
        Ok(())
    }

    // fn write_to_socket(&mut self, message: &[u8]) -> Result<(), CommandError> {
    //     self.socket
    //         .write_all(message)
    //         .map_err(|error| CommandError::SendingMessage(error.to_string()))?;
    //     Ok(())
    // }

    fn write_in_tpk_to_socket(&mut self, line: &str) -> Result<(), CommandError> {
        let line = line.to_string().to_pkt_format();
        self.write_string_to_socket(&line)
    }

    /// Lee los objetos del socket, primero lee el header del packfile y luego lee los objetos
    fn read_objects(&mut self) -> Result<Vec<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
        let object_number = self.read_packfile_header()?;
        let objects_data = self.read_objects_in_packfile(object_number)?;
        Ok(objects_data)
    }

    /// Lee todos los objetos del packfile y devuelve un vector que contiene tuplas con:\
    /// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
    fn read_objects_in_packfile(
        &mut self,
        object_number: u32,
    ) -> Result<Vec<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
        let mut objects_data = Vec::new();
        let mut buffed_reader = TcpStreamBuffedReader::new(&self.socket);

        for _ in 0..object_number {
            let mut buffed_reader: &mut TcpStreamBuffedReader<'_> = &mut buffed_reader;
            let (object_type, len) = read_object_header_from_packfile(buffed_reader)?;

            buffed_reader.clean_up_to_pos();
            let mut decoder = flate2::read::ZlibDecoder::new(&mut buffed_reader);
            let mut deflated_data = Vec::new();

            decoder
                .read_to_end(&mut deflated_data)
                .map_err(|_| CommandError::ErrorExtractingPackfile)?;
            let bytes_used = decoder.total_in() as usize;
            buffed_reader.set_pos(bytes_used);

            let object = deflated_data;
            objects_data.push((object_type, len, object));
        }
        Ok(objects_data)
    }

    fn read_packfile_header(&mut self) -> Result<u32, CommandError> {
        let signature = self.read_pack_signature()?;
        if signature != "PACK" {
            return Err(CommandError::ErrorReadingPkt);
        }
        let version = self.read_version_number()?;
        if version != 2 {
            return Err(CommandError::ErrorReadingPkt);
        }
        let object_number = self.read_object_number()?;
        Ok(object_number)
    }

    /// lee la firma del packfile, los primeros 4 bytes del socket
    fn read_pack_signature(&mut self) -> Result<String, CommandError> {
        let signature_buf = &mut [0; 4];
        self.socket
            .read_exact(signature_buf)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        let signature = String::from_utf8(signature_buf.to_vec())
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        Ok(signature)
    }

    /// lee la versión del packfile, los siguientes 4 bytes del socket
    fn read_version_number(&mut self) -> Result<u32, CommandError> {
        let mut version_buf = [0; 4];
        self.socket
            .read_exact(&mut version_buf)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        let version = u32::from_be_bytes(version_buf);
        Ok(version)
    }

    /// lee la cantidad de objetos en el packfile, los siguientes 4 bytes del socket
    fn read_object_number(&mut self) -> Result<u32, CommandError> {
        let mut object_number_buf = [0; 4];
        self.socket
            .read_exact(&mut object_number_buf)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        let object_number = u32::from_be_bytes(object_number_buf);
        Ok(object_number)
    }
}

fn read_object_header_from_packfile(
    buffed_reader: &mut TcpStreamBuffedReader<'_>,
) -> Result<(PackfileObjectType, usize), CommandError> {
    let mut first_byte_buf = [0; 1];
    buffed_reader
        .read_exact(&mut first_byte_buf)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;

    let object_type_u8 = first_byte_buf[0] >> 4 & 0b00000111;
    let object_type = PackfileObjectType::from_u8(object_type_u8)?;

    let mut bits = Vec::new();
    let first_byte_buf_len_bits = first_byte_buf[0] & 0b00001111;

    let mut bit_chunk = Vec::new();
    for i in (0..4).rev() {
        let bit = (first_byte_buf_len_bits >> i) & 1;
        bit_chunk.push(bit);
    }

    bits.splice(0..0, bit_chunk);
    let mut is_last_byte: bool = first_byte_buf[0] >> 7 == 0;
    while !is_last_byte {
        let mut seven_bit_chunk = Vec::<u8>::new();
        let mut current_byte_buf = [0; 1];
        buffed_reader
            .read_exact(&mut current_byte_buf)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        let current_byte = current_byte_buf[0];
        let seven_bit_chunk_with_zero = current_byte & 0b01111111;
        for i in (0..7).rev() {
            let bit = (seven_bit_chunk_with_zero >> i) & 1;
            seven_bit_chunk.push(bit);
        }
        bits.splice(0..0, seven_bit_chunk);
        is_last_byte = current_byte >> 7 == 0;
    }

    let len = bits_to_usize(&bits);
    Ok((object_type, len))
}

fn get_response(mut socket: &TcpStream) -> Result<Vec<String>, CommandError> {
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

fn bits_to_usize(bits: &[u8]) -> usize {
    let mut result = 0;
    let max_power = bits.len() - 1;
    for (i, bit) in bits.iter().enumerate() {
        if *bit == 1 {
            let exp = max_power - i;
            result += 2usize.pow(exp as u32);
        }
    }
    result
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
        let input_len = self.len() + 4;
        let input_len_hex = format!("{:04x}", input_len);
        let output = input_len_hex + self;
        output
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
            .map_err(|_| CommandError::ErrorReadingPkt)?;
        let line = String::from_utf8(line_buffer).map_err(|_| CommandError::ErrorReadingPkt)?;
        Ok(Some(line))
    }
}
