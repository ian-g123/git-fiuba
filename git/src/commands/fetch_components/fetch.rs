//! Se conecta mediante TCP a la dirección asignada por argv.
//! Lee lineas desde stdin y las manda mediante el socket.

use std::io::stdin;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

fn main() -> Result<(), ()> {
    let address = "127.1.0.0:9418";
    println!("Conectándome a {:?}", address);

    client_run(&address, &mut stdin()).unwrap();
    Ok(())
}

fn pkt_format(input: &str) -> String {
    let input_len = input.len() + 4 +1 /* por el \n*/;
    let input_len_hex = format!("{:04x}", input_len);
    let mut output = input_len_hex + input;
    output.push('\n');
    output
}

fn client_run(address: &str, stream: &mut dyn Read) -> std::io::Result<()> {
    let socket = TcpStream::connect(address)?;

    let line = "git-upload-pack /server-repo\0host=127.1.0.1\0\0version=1\0";
    println!("Enviando {:?}", line);
    let lines = send(line, &socket)?;
    println!("=========\nRecibido:");
    for line in &lines {
        println!("{:?}", line);
    }
    let (head_branch, _) = lines.last().unwrap().split_once(' ').unwrap();
    println!("HEAD branch: {:?}", head_branch);

    let line = format!("want {}", head_branch);
    println!("Enviando {:?}", line);
    let lines = send_done(&line, &socket)?;
    println!("=========\nRecibido:");
    for line in &lines {
        println!("{:?}", line);
    }
    Ok(())
}

fn send(line: &str, mut socket: &TcpStream) -> Result<Vec<String>, std::io::Error> {
    let line = &pkt_format(line);
    print!("|| Sending: {}", line);
    socket.write_all(line.as_bytes())?;
    let mut lines = Vec::<String>::new();
    loop {
        let mut size_buffer = [0; 4];
        socket.read(&mut size_buffer).unwrap();
        let size_vec =
            hex_string_to_u8_vec(String::from_utf8(size_buffer.to_vec()).unwrap().as_str());
        let size: usize = u16::from_be_bytes(size_vec) as usize;
        if size == 0 {
            break;
        }
        let mut line_buffer = vec![0; size - 4];
        socket.read_exact(&mut line_buffer).unwrap();
        let line = String::from_utf8(line_buffer).unwrap();
        lines.push(line);
    }
    Ok(lines)
}

fn send_done(line: &str, mut socket: &TcpStream) -> Result<Vec<String>, std::io::Error> {
    let line = &pkt_format(line);
    print!("|| Sending: {}", line);
    socket.write_all((line).as_bytes())?;
    socket.write_all("0000".as_bytes())?;
    socket.write_all("0009done\n".as_bytes())?;
    let mut lines = Vec::<String>::new();
    loop {
        let mut size_buffer = [0; 4];
        socket.read(&mut size_buffer).unwrap();
        let size_vec =
            hex_string_to_u8_vec(String::from_utf8(size_buffer.to_vec()).unwrap().as_str());
        let size: usize = u16::from_be_bytes(size_vec) as usize;
        if size == 0 {
            break;
        }
        let mut line_buffer = vec![0; size - 4];
        socket.read_exact(&mut line_buffer).unwrap();
        let line = String::from_utf8(line_buffer).unwrap();
        println!("pushing: {:?}", line);
        lines.push(line.clone());
        if line == "NAK\n" {
            break;
        }
    }
    read_package(socket);
    Ok(lines)
}

fn read_package(mut socket: &TcpStream) {
    let signature = read_signature(socket);
    println!("signature: {:?}", signature);
    let version = read_verson_number(socket);
    println!("version: {:?}", version);
    let object_number = read_object_number(socket);
    println!("object_number: {:?}", object_number);

    leer_bits(socket);

    // The header is followed by number of object entries, each of which looks like this:

    // (undeltified representation)
    // n-byte type and length (3-bit type, (n-1)*7+4-bit length)
    // compressed data

    //    (deltified representation)
    //    n-byte type and length (3-bit type, (n-1)*7+4-bit length)
    //    base object name if OBJ_REF_DELTA or a negative relative
    // offset from the delta object's position in the pack if this
    // is an OBJ_OFS_DELTA object
    //    compressed delta data

    // Observation: length of each object is encoded in a variable
    // length format and is not constrained to 32-bit or anything.

    // leer_objetos(object_number, socket);
}

fn leer_bits(mut socket: &TcpStream) {
    let mut buf = &mut vec![0; 300];
    socket.read(&mut buf).unwrap();
    // print bits
    let bits = concat_bytes_to_bits(&buf);
    for (i, bit) in bits.iter().enumerate() {
        if i % 8 == 0 {
            print!(" ");
        }
        print!("{}", bit);
    }
}

fn leer_objetos(object_number: u32, mut socket: &TcpStream) {
    for _ in 0..object_number {
        // Object types
        // Valid object types are:
        // OBJ_COMMIT (1)
        // OBJ_TREE (2)
        // OBJ_BLOB (3)
        // OBJ_TAG (4)
        // OBJ_OFS_DELTA (6)
        // OBJ_REF_DELTA (7)
        let mut first_byte_buf = [0; 1];
        socket.read_exact(&mut first_byte_buf).unwrap();
        // Object type is three bits
        println!("first_byte_buf: {:?}", first_byte_buf);
        let object_type = first_byte_buf[0] >> 5;
        println!("object_type: {:?}", object_type);
        let object_type_str = match object_type {
            1 => "OBJ_COMMIT",
            2 => "OBJ_TREE",
            3 => "OBJ_BLOB",
            4 => "OBJ_TAG",
            6 => "OBJ_OFS_DELTA",
            7 => "OBJ_REF_DELTA",
            _ => panic!("Invalid object type"),
        };
        println!("object_type: ({}) {:?}", object_type, object_type_str);

        let mut bits = Vec::new();
        let first_byte_buf_len_bits = first_byte_buf[0] & 0b00001111;
        let mut seven_bit_chunk = Vec::new();
        for i in (0..4).rev() {
            let bit = (first_byte_buf_len_bits >> i) & 1;
            seven_bit_chunk.push(bit);
        }
        println!("byte_fraction: {:?}", seven_bit_chunk);
        bits.splice(0..0, seven_bit_chunk);

        let mut is_last_byte: bool = (first_byte_buf[0] >> 4) & 1 == 0;
        while is_last_byte {
            let mut seven_bit_chunk = Vec::<u8>::new();
            let current_byte = next_byte(socket);
            is_last_byte = current_byte >> 7 == 0;
            let seven_bit_chunk_with_zero = current_byte & 0b01111111;
            for i in (0..7).rev() {
                let bit = (seven_bit_chunk_with_zero >> i) & 1;
                seven_bit_chunk.push(bit);
            }
            println!("byte_fraction: {:?}", seven_bit_chunk);
            bits.splice(0..0, seven_bit_chunk);
        }
        println!("bits: {:?}", bits);
        let len = bits_to_usize(&bits);
        println!("len: {:?}", len);

        let mut buf = &mut vec![0; len];
        socket.read(&mut buf).unwrap();
        println!("buf: {:?}", buf);
    }
}

fn read_signature(mut socket: &TcpStream) -> String {
    let signature_buf = &mut [0; 4];
    socket.read_exact(signature_buf).unwrap();
    let signature = String::from_utf8(signature_buf.to_vec()).unwrap();
    signature
}

fn read_verson_number(mut socket: &TcpStream) -> u32 {
    let mut version_buf = [0; 4];
    socket.read_exact(&mut version_buf).unwrap();
    let version = u32::from_be_bytes(version_buf);
    version
}

fn read_object_number(mut socket: &TcpStream) -> u32 {
    let mut object_number_buf = [0; 4];
    socket.read_exact(&mut object_number_buf).unwrap();
    let object_number = u32::from_be_bytes(object_number_buf);
    object_number
}

fn next_byte(mut socket: &TcpStream) -> u8 {
    let mut next_byte_buf = [0; 1];
    socket.read_exact(&mut next_byte_buf).unwrap();
    let next_byte = next_byte_buf[0];
    next_byte
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

fn concat_bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in (0..8).rev() {
            let bit = (byte >> i) & 1;
            bits.push(bit);
        }
    }
    bits
}

pub fn hex_string_to_u8_vec(hex_string: &str) -> [u8; 2] {
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
