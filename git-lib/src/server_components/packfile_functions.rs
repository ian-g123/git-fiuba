use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
};

use crate::{
    command_errors::CommandError,
    file_compressor::compress,
    objects::{
        commit_object::CommitObject,
        git_object::{GitObject, GitObjectTrait},
        tree::Tree,
    },
    server_components::packfile_object_type::PackfileObjectType,
    utils::{aux::get_sha1, super_string::u8_vec_to_hex_string},
};

use super::reader::TcpStreamBuffedReader;

pub fn get_objects_from_tree(
    hash_objects: &mut HashMap<String, GitObject>,
    tree: &Tree,
) -> Result<(), CommandError> {
    for (_object_name, (object_hash, git_object_opt)) in tree.get_objects() {
        let mut git_object = git_object_opt.ok_or(CommandError::ShallowTree)?;
        if let Some(son_tree) = git_object.as_tree() {
            get_objects_from_tree(hash_objects, &son_tree)?;
        }
        let object_hash_str = u8_vec_to_hex_string(&object_hash);
        hash_objects.insert(object_hash_str, git_object);
    }
    Ok(())
}

pub fn packfile_header(objects_number: u32) -> Vec<u8> {
    let mut header = Vec::<u8>::new();
    header.extend("PACK".as_bytes());
    header.extend(2u32.to_be_bytes());
    header.extend(objects_number.to_be_bytes());
    header
}

pub fn write_object_to_packfile(
    mut git_object: GitObject,
    packfile: &mut Vec<u8>,
) -> Result<(), CommandError> {
    let object_content = git_object.content(None)?;
    // let mut cursor = Cursor::new(&mut object_content);
    // git_object.write_to(&mut cursor)?;

    let type_str = git_object.type_str();
    println!("type_str: {:?}", type_str);
    println!(
        "object_content: {:?}",
        String::from_utf8_lossy(&object_content)
    );
    let object_len = object_content.len();

    let compressed_object = compress(&object_content)?;
    let pf_type = PackfileObjectType::from_str(type_str.as_str())?;

    let mut len_temp = object_len;
    let first_four = (len_temp & 0b00001111) as u8;
    len_temp >>= 4;
    let mut len_bytes: Vec<u8> = Vec::new();
    if len_temp != 0 {
        loop {
            let mut byte = (len_temp & 0b01111111) as u8;
            len_temp >>= 7;
            if len_temp == 0 {
                len_bytes.push(byte);
                break;
            }
            byte |= 0b10000000;
            len_bytes.push(byte);
        }
    }

    let type_and_len_byte =
        (pf_type.to_u8()) << 4 | first_four | if len_bytes.is_empty() { 0 } else { 0b10000000 };
    println!("writing: {:?}", &type_and_len_byte);
    println!("object_len: {:?}", object_len);
    println!("writing: {:?}", &len_bytes);
    println!("writing: {:?}", String::from_utf8_lossy(&compressed_object));
    packfile.push(type_and_len_byte);
    packfile.extend(len_bytes);
    packfile.extend(compressed_object);
    Ok(())
}

pub fn make_packfile(
    commits_map: HashMap<String, (CommitObject, Option<String>)>, // HashMap<hash, (CommitObject, Option<branch>)>
) -> Result<Vec<u8>, CommandError> {
    let mut hash_objects: HashMap<String, GitObject> = HashMap::new();

    for (hash_commit, (commit_object, _branch)) in commits_map {
        let Some(tree) = commit_object.get_tree() else {
            return Err(CommandError::PushTreeError);
        };
        let mut tree_owned = tree.to_owned();
        get_objects_from_tree(&mut hash_objects, tree)?;
        hash_objects.insert(hash_commit, Box::new(commit_object));
        hash_objects.insert(
            tree_owned.get_hash_string()?,
            Box::new(tree_owned.to_owned()),
        );
    }
    println!("hash_objects: {:?}", hash_objects.keys());

    let mut packfile: Vec<u8> = Vec::new();
    let packfile_header = packfile_header(hash_objects.len() as u32);
    println!(
        "packfile_header: {:?}",
        String::from_utf8_lossy(&packfile_header)
    );
    packfile.write(&packfile_header).map_err(|error| {
        CommandError::FileWriteError(format!("Error escribiendo en packfile: {}", error))
    })?;
    for (_hash_object, git_object) in hash_objects {
        write_object_to_packfile(git_object, &mut packfile)?;
    }
    packfile.write(&get_sha1(&packfile)).map_err(|error| {
        CommandError::FileWriteError(format!("Error escribiendo en packfile: {}", error))
    })?;

    Ok(packfile)
}

/// lee la firma del packfile, los primeros 4 bytes del socket
fn read_pack_signature(socket: &mut TcpStream) -> Result<String, CommandError> {
    let signature_buf = &mut [0; 4];
    socket
        .read_exact(signature_buf)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let signature = String::from_utf8(signature_buf.to_vec())
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    Ok(signature)
}

/// lee la versión del packfile, los siguientes 4 bytes del socket
fn read_version_number(socket: &mut TcpStream) -> Result<u32, CommandError> {
    let mut version_buf = [0; 4];
    socket
        .read_exact(&mut version_buf)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let version = u32::from_be_bytes(version_buf);
    Ok(version)
}

/// lee la cantidad de objetos en el packfile, los siguientes 4 bytes del socket
fn read_object_number(socket: &mut TcpStream) -> Result<u32, CommandError> {
    let mut object_number_buf = [0; 4];
    socket
        .read_exact(&mut object_number_buf)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let object_number = u32::from_be_bytes(object_number_buf);
    Ok(object_number)
}

fn read_packfile_header(socket: &mut TcpStream) -> Result<u32, CommandError> {
    let signature = read_pack_signature(socket)?;
    if signature != "PACK" {
        return Err(CommandError::ErrorReadingPkt);
    }
    let version = read_version_number(socket)?;
    if version != 2 {
        return Err(CommandError::ErrorReadingPkt);
    }
    let object_number = read_object_number(socket)?;
    Ok(object_number)
}

/// Lee todos los objetos del packfile y devuelve un vector que contiene tuplas con:\
/// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
fn read_objects_in_packfile(
    socket: &mut TcpStream,
    object_number: u32,
) -> Result<Vec<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let mut objects_data = Vec::new();
    let mut buffed_reader = TcpStreamBuffedReader::new(socket);

    for _ in 0..object_number {
        let mut buffed_reader: &mut TcpStreamBuffedReader<'_> = &mut buffed_reader;
        let (object_type, len) = read_object_header_from_packfile(buffed_reader)?;
        if object_type == PackfileObjectType::RefDelta
            || object_type == PackfileObjectType::OfsDelta
        {
            todo!("❌ Delta objects not implemented");
        }
        buffed_reader.clean_up_to_pos();
        let mut decoder = flate2::read::ZlibDecoder::new(&mut buffed_reader);
        let mut object_content = Vec::new();

        decoder
            .read_to_end(&mut object_content)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        let bytes_used = decoder.total_in() as usize;
        buffed_reader.set_pos(bytes_used);

        if object_content.len() != len {
            return Err(CommandError::ErrorDecompressingObject(format!(
                "Expected length: {}, Decompressed data length: {}",
                len,
                object_content.len()
            )));
        }

        objects_data.push((object_type, len, object_content));
    }
    Ok(objects_data)
}

pub fn read_object_header_from_packfile(
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

/// Lee los objetos del socket, primero lee el header del packfile y luego lee los objetos
pub fn read_objects(
    socket: &mut TcpStream,
) -> Result<Vec<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let object_number = read_packfile_header(socket)?;
    let objects_data = read_objects_in_packfile(socket, object_number)?;
    Ok(objects_data)
}
