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

    let line = "want b102639443a98b76ff60aa8404e79c44667ee8b2";
    println!("Enviando {:?}", line);
    let lines = send_done(line, &socket)?;
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
    let signature_buf = &mut [0; 4];
    socket.read_exact(signature_buf).unwrap();
    let signature = String::from_utf8(signature_buf.to_vec()).unwrap();
    println!("signature: {:?}", signature);
    let mut version_buf = [0; 4];
    socket.read_exact(&mut version_buf).unwrap();
    let version = u32::from_be_bytes(version_buf);
    println!("version: {:?}", version);
    let mut object_number_buf = [0; 4];
    socket.read_exact(&mut object_number_buf).unwrap();
    let object_number = u32::from_be_bytes(object_number_buf);
    println!("object_number: {:?}", object_number);

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

    //==

    // let mut buf = [0; 250];
    // socket.read(&mut buf).unwrap();
    // // Print bits
    // let mut bits = Vec::with_capacity(buf.len() * 8);
    // for byte in &buf {
    //     for i in (0..8).rev() {
    //         let bit = (byte >> i) & 1;
    //         bits.push(bit);
    //     }
    // }
    // println!("bits: {:?}", bits);

    //==

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

        let object_type = first_byte_buf[0] >> 5;
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
        let mut current_byte = first_byte_buf[0];
        loop {
            let mut last_len_bit_group = current_byte >> 4 & 1;
            let mut first_byte_buf_len_bits = current_byte & 0b00001111;
            let mut next_byte_buf = [0; 1];
            socket.read_exact(&mut next_byte_buf).unwrap();
            let next_byte = next_byte_buf[0];
            for i in (0..4).rev() {
                let bit = (first_byte_buf_len_bits >> i) & 1;
                bits.push(bit);
            }
            for i in (0..3).rev() {
                let bit = (next_byte >> i) & 1;
                bits.push(bit);
            }
            if last_len_bit_group == 0 {
                break;
            }
            current_byte = next_byte;
        }
        println!("bits: {:?}", bits);
        let len = bits_to_u32(&bits);
    }
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
    println!("hex_string: {:?}", hex_string);
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
