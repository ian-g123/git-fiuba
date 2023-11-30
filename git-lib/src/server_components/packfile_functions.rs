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
    let mut packfile: Vec<u8> = Vec::new();
    let packfile_header = packfile_header(hash_objects.len() as u32);
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

/// Lee todos los objetos del packfile y devuelve un vector que contiene tuplas con:\
/// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
fn read_object_from_offset(
    socket: &mut dyn Read,
    offset: u32,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    let mut previous_objects = vec![0; offset as usize];
    socket
        .read_exact(&mut previous_objects)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;

    let (object_type, len) = read_object_header_from_packfile(socket)?;
    if object_type == PackfileObjectType::RefDelta || object_type == PackfileObjectType::OfsDelta {
        todo!("❌ Delta objects not implemented");
    }

    let mut decoder = flate2::read::ZlibDecoder::new(socket);
    let mut object_content = Vec::new();

    decoder
        .read_to_end(&mut object_content)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;

    Ok((object_type, len, object_content))
}

pub fn read_object_header_from_packfile(
    buffed_reader: &mut dyn Read,
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

fn get_object_packfile_offset(
    sha1: &[u8; 20],
    index_file: &mut dyn Read,
) -> Result<Option<u32>, CommandError> {
    // The first four bytes are always 255, 116, 79, 99
    let mut header_bytes = [0; 4];
    index_file
        .read_exact(&mut header_bytes)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    if header_bytes != [255, 116, 79, 99] {
        return Err(CommandError::ErrorExtractingPackfile);
    }
    // The next four bytes denote the version number
    let mut version_bytes = [0; 4];
    index_file
        .read_exact(&mut version_bytes)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    if version_bytes != [0, 0, 0, 2] {
        return Err(CommandError::ErrorExtractingPackfile);
    }
    // # Fanout table
    // The first level of entries in this table is a series of 256 entries of four bytes
    // each, 1024 bytes long in total. According to the documentation, “[the] N-th entry
    // of this table records the number of objects in the corresponding pack, the first
    // byte of whose object name is less than or equal to N.”
    let mut cumulative_objects_counts: Vec<u8> = vec![0; 1024];
    index_file
        .read_exact(&mut cumulative_objects_counts)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let object_group_index = sha1[0] as usize;
    let prev_object_group_count = u32::from_be_bytes(
        cumulative_objects_counts[(object_group_index - 1) * 4..object_group_index * 4]
            .to_vec()
            .try_into()
            .map_err(|_| CommandError::ErrorExtractingPackfile)?,
    );
    let object_group_count = u32::from_be_bytes(
        cumulative_objects_counts[object_group_index * 4..(object_group_index + 1) * 4]
            .to_vec()
            .try_into()
            .map_err(|_| CommandError::ErrorExtractingPackfile)?,
    );
    let object_count = u32::from_be_bytes(
        cumulative_objects_counts[cumulative_objects_counts.len() - 4..]
            .to_vec()
            .try_into()
            .map_err(|_| CommandError::ErrorExtractingPackfile)?,
    );

    // The second layer of the fanout table contains the 20-byte object names, in order.
    // We already know how many to expect from the first layer of the fanout table.
    let mut prev_group_object_names = vec![0; prev_object_group_count as usize * 20];
    index_file
        .read_exact(&mut prev_group_object_names)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let mut number_objets_read_in_group = 0;
    let mut found = false;
    for _ in 0..object_group_count {
        let mut object_name = [0; 20];
        index_file
            .read_exact(&mut object_name)
            .map_err(|_| CommandError::ErrorExtractingPackfile)?;
        if object_name == *sha1 {
            found = true;
            break;
        }
        number_objets_read_in_group += 1;
    }
    if !found {
        return Ok(None);
    }
    let index_of_object = prev_object_group_count + number_objets_read_in_group;
    let mut rest_objets = vec![0; (object_count - index_of_object - 1) as usize * 20];
    index_file
        .read_exact(&mut rest_objets)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;

    // The third layer of the fanout table gives us a four-byte cyclic redundancy check value for each object.
    let mut crcs = vec![0; object_count as usize * 4];
    index_file
        .read_exact(&mut crcs)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;

    // The fourth layer contains the information we’ve been looking for: the packfile offsets for each object.
    // These are also four bytes per entry.
    let mut prev_offsets = vec![0; index_of_object as usize * 4];
    index_file
        .read_exact(&mut prev_offsets)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let mut offset_bytes = vec![0; 4];
    index_file
        .read_exact(&mut offset_bytes)
        .map_err(|_| CommandError::ErrorExtractingPackfile)?;
    let offset = u32::from_be_bytes(
        offset_bytes
            .try_into()
            .map_err(|_| CommandError::ErrorExtractingPackfile)?,
    );

    Ok(Some(offset))
}

pub fn search_object_from_hash(
    sha1: [u8; 20],
    mut index_file: &mut dyn Read,
    mut packfile: &mut dyn Read,
) -> Result<Option<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let Some(packfile_offset) = get_object_packfile_offset(&sha1, &mut index_file)? else {
        return Ok(None);
    };
    let (obj_type, obj_len, content) = read_object_from_offset(&mut packfile, packfile_offset)?;
    Ok(Some((obj_type, obj_len, content)))
}

#[test]
fn test_read_index_with_three_objects() {
    let index_file_bits = vec![
        0xff, 0x74, 0x4f, 0x63, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00,
        0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03,
        0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
        0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
        0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00,
        0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03,
        0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03, 0xdb, 0x96, 0x35,
        0x15, 0x88, 0x0f, 0x98, 0xc8, 0x43, 0x5d, 0xf2, 0xce, 0x90, 0x2e, 0x4f, 0x33, 0x7a, 0xac,
        0x7e, 0xa6, 0xdf, 0x2b, 0x8f, 0xc9, 0x9e, 0x1c, 0x1d, 0x4d, 0xbc, 0x0a, 0x85, 0x4d, 0x9f,
        0x72, 0x15, 0x7f, 0x1d, 0x6e, 0xa0, 0x78, 0xe6, 0x9d, 0xe2, 0x9b, 0xb2, 0xd1, 0xd6, 0x43,
        0x4b, 0x8b, 0x29, 0xae, 0x77, 0x5a, 0xd8, 0xc2, 0xe4, 0x8c, 0x53, 0x91, 0xec, 0x39, 0x06,
        0x28, 0xc1, 0xc7, 0x9c, 0xe6, 0x6e, 0x76, 0x00, 0x29, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00,
        0x00, 0x8f, 0x00, 0x00, 0x00, 0xba, 0x10, 0x09, 0xae, 0x88, 0xdd, 0xdd, 0x13, 0x96, 0xeb,
        0x58, 0x63, 0xa7, 0x76, 0x36, 0xf7, 0x28, 0x00, 0x2c, 0x8d, 0xff, 0xf2, 0xcf, 0x0d, 0x74,
        0x77, 0xb4, 0x9b, 0x1a, 0x03, 0xcc, 0xc4, 0x08, 0x33, 0xab, 0xff, 0x6e, 0xae, 0x17, 0xe2,
        0x4f,
    ];
    let packfile_bits = vec![
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x94, 0x0e, 0x78,
        0x9c, 0xad, 0xcc, 0xb1, 0x0e, 0xc2, 0x20, 0x10, 0x00, 0xd0, 0x9d, 0xaf, 0xe0, 0x07, 0x34,
        0x5c, 0x29, 0x42, 0x13, 0xd3, 0xf8, 0x2b, 0x57, 0xee, 0xd0, 0x4b, 0x2c, 0x18, 0x7a, 0x2c,
        0x7e, 0xbd, 0x83, 0x93, 0xbb, 0xe3, 0x5b, 0x9e, 0x76, 0x66, 0x4b, 0x65, 0xda, 0x52, 0xc9,
        0xcb, 0xc2, 0x90, 0x81, 0x66, 0xda, 0xb2, 0xc3, 0x14, 0x66, 0x5a, 0x4a, 0x9c, 0x20, 0xc4,
        0x02, 0x74, 0x61, 0x74, 0x31, 0x19, 0x1c, 0xfa, 0x68, 0xdd, 0xbe, 0x50, 0xbb, 0x64, 0x69,
        0xda, 0x46, 0xaf, 0x6c, 0xaf, 0xbf, 0xbe, 0xf1, 0xa1, 0x83, 0x04, 0xab, 0xf2, 0xd1, 0xfa,
        0x1d, 0xab, 0xbc, 0x91, 0xda, 0x71, 0xc6, 0xbe, 0x5a, 0x88, 0x0e, 0x7c, 0x84, 0x18, 0xbc,
        0x3d, 0x39, 0xef, 0x9c, 0xc9, 0x6d, 0xdf, 0x45, 0x95, 0xff, 0x9a, 0x1a, 0xa9, 0xa2, 0x82,
        0xcf, 0x6f, 0x6e, 0x3e, 0x01, 0x71, 0x4e, 0x3a, 0xa0, 0x02, 0x78, 0x9c, 0x33, 0x34, 0x30,
        0x30, 0x33, 0x31, 0x51, 0x48, 0xcb, 0xcc, 0x49, 0x65, 0x78, 0x36, 0xf7, 0xd1, 0xec, 0x4d,
        0x17, 0xaf, 0x39, 0x7b, 0x77, 0x6b, 0xae, 0x2b, 0x8f, 0xba, 0x71, 0xe8, 0x49, 0x4f, 0xf0,
        0x44, 0x00, 0xd1, 0x5a, 0x0e, 0xf8, 0x30, 0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x10, 0x09, 0xae, 0x88, 0xdd, 0xdd, 0x13, 0x96, 0xeb, 0x58, 0x63, 0xa7, 0x76, 0x36, 0xf7,
        0x28, 0x00, 0x2c, 0x8d, 0xff,
    ];

    fn test_read_index(
        index_file_bits: &Vec<u8>,
        packfile_bits: &Vec<u8>,
        hash: &str,
        expected_obj_type: PackfileObjectType,
        expected_obj_len: usize,
        expected_content: Option<Vec<u8>>,
    ) {
        let mut index_file = std::io::Cursor::new(index_file_bits);

        let mut packfile = std::io::Cursor::new(packfile_bits);
        let sha1 = crate::utils::aux::hex_string_to_u8_vec(hash);
        let (obj_type, obj_len, content) =
            search_object_from_hash(sha1, &mut index_file, &mut packfile)
                .unwrap()
                .unwrap();
        assert_eq!(obj_type, expected_obj_type);
        assert_eq!(obj_len, expected_obj_len);
        if let Some(expected_content) = expected_content {
            assert_eq!(content, expected_content);
        }
    }

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "db963515880f98c8435df2ce902e4f337aac7ea6",
        PackfileObjectType::Commit,
        228,
        None,
    );

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "df2b8fc99e1c1d4dbc0a854d9f72157f1d6ea078",
        PackfileObjectType::Tree,
        32,
        None,
    );

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        PackfileObjectType::Blob,
        0,
        Some(vec![]),
    );
}
