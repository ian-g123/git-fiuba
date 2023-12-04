use std::{
    collections::HashMap,
    fmt::format,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    net::TcpStream,
};

use crate::{
    command_errors::CommandError,
    file_compressor::compress,
    logger::{self, Logger},
    objects::{
        commit_object::CommitObject,
        git_object::{
            git_object_from_data, write_to_stream_from_content, GitObject, GitObjectTrait,
        },
        tree::Tree,
    },
    objects_database::ObjectsDatabase,
    server_components::{
        delta_instructions::delta_instruction::read_delta_instruction_from,
        packfile_object_type::PackfileObjectType,
    },
    utils::{aux::get_sha1, super_string::u8_vec_to_hex_string},
};

use super::reader::BuffedReader;

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
    commits_map: HashMap<String, (CommitObject, usize, usize)>, // HashMap<hash, (CommitObject, Option<branch>)>
) -> Result<Vec<u8>, CommandError> {
    let mut hash_objects: HashMap<String, GitObject> = HashMap::new();

    for (hash_commit, (commit_object, _branch, _index)) in commits_map {
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
fn read_pack_signature(socket: &mut BuffedReader) -> Result<String, CommandError> {
    let signature_buf = &mut [0; 4];
    socket
        .read_exact(signature_buf)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let signature = String::from_utf8(signature_buf.to_vec())
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    Ok(signature)
}

/// lee la versión del packfile, los siguientes 4 bytes del socket
fn read_version_number(socket: &mut BuffedReader) -> Result<u32, CommandError> {
    let mut version_buf = [0; 4];
    socket
        .read_exact(&mut version_buf)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let version = u32::from_be_bytes(version_buf);
    Ok(version)
}

/// lee la cantidad de objetos en el packfile, los siguientes 4 bytes del socket
fn read_object_number(socket: &mut BuffedReader) -> Result<u32, CommandError> {
    let mut object_number_buf = [0; 4];
    socket
        .read_exact(&mut object_number_buf)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let object_number = u32::from_be_bytes(object_number_buf);
    Ok(object_number)
}

fn read_packfile_header(buffed_reader: &mut BuffedReader) -> Result<u32, CommandError> {
    let signature = read_pack_signature(buffed_reader)?;
    if signature != "PACK" {
        return Err(CommandError::ErrorReadingPkt);
    }
    let version = read_version_number(buffed_reader)?;
    if version != 2 {
        return Err(CommandError::ErrorReadingPkt);
    }
    let object_number = read_object_number(buffed_reader)?;
    Ok(object_number)
}

/// Lee todos los objetos del packfile y devuelve un vector que contiene tuplas con:\
/// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
fn read_objects_from_packfile_body(
    buffed_reader: &mut BuffedReader,
    exp_obj_number: u32,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let mut objects_data = HashMap::<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>::new();
    for i in 0..exp_obj_number {
        logger.log(&format!(
            "Reading object number: {}/{}",
            i + 1,
            exp_obj_number
        ));
        let (obj_type, len, object_content) =
            read_one_object_data_from_packfile(buffed_reader, db, logger, Some(&objects_data))?;
        let mut hashable_content = Vec::new();
        write_to_stream_from_content(
            &mut hashable_content,
            object_content.clone(),
            obj_type.to_string(),
        )?;
        let sha1 = get_sha1(&hashable_content);

        if object_content.len() != len {
            return Err(CommandError::ErrorDecompressingObject(format!(
                "Expected length: {}, Decompressed data length: {}",
                len,
                object_content.len()
            )));
        }

        objects_data.insert(sha1, (obj_type, len, object_content));
    }
    Ok(objects_data)
}

/// Lee un objecto del packfile y devuelve una tuplas con:\
/// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
fn read_object_from_offset(
    buffed_reader: &mut BuffedReader,
    offset: u32,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    buffed_reader.record_and_fast_foward_to(offset as usize, logger)?;
    read_one_object_data_from_packfile(buffed_reader, db, logger, None)
}

fn read_one_object_data_from_packfile(
    buffed_reader: &mut BuffedReader,
    db: &ObjectsDatabase,
    logger: &mut Logger,
    objects_data: Option<&HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>>,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    let offset = buffed_reader.get_pos() as u32;
    let (obj_type, len) = read_object_header_from_packfile(buffed_reader)?;
    match obj_type {
        PackfileObjectType::RefDelta => {
            read_ref_delta(buffed_reader, objects_data, db, len, logger)
        }
        PackfileObjectType::OfsDelta => read_ofs_delta(buffed_reader, offset, db, len, logger),
        _ => read_undeltified_object(buffed_reader, len, obj_type, logger),
    }
}

fn read_undeltified_object(
    buffed_reader: &mut BuffedReader,
    len: usize,
    obj_type: PackfileObjectType,
    logger: &mut Logger,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    logger.log(&format!("Reading Undeltified Object"));
    let object_content = read_extract_rewind(buffed_reader, len, logger)?;
    Ok((obj_type, len, object_content))
}

fn read_ofs_delta(
    buffed_reader: &mut BuffedReader,
    offset: u32,
    db: &ObjectsDatabase,
    len: usize,
    logger: &mut Logger,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    logger.log(&format!("Reading Delta Offset: {}", offset));
    let base_obj_neg_offset = read_ofs(buffed_reader)?;
    let base_object_offset = offset - base_obj_neg_offset;
    logger.log("Reading base object");
    let current_position = buffed_reader.get_pos();
    let (base_obj_type, base_obj_len, base_obj_content) =
        read_object_from_offset(buffed_reader, base_object_offset, db, logger)?;
    buffed_reader.record_and_fast_foward_to(current_position, logger)?;

    read_and_apply_delta_instructions(
        buffed_reader,
        len,
        base_obj_type,
        base_obj_len,
        base_obj_content,
        logger,
    )
}

fn read_ref_delta(
    buffed_reader: &mut BuffedReader,
    objects_data: Option<&HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>>,
    db: &ObjectsDatabase,
    len: usize,
    logger: &mut Logger,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    logger.log("Reading Ref Delta");
    let (base_obj_type, base_obj_len, base_obj_content) =
        read_ref_base_object(buffed_reader, objects_data, db, logger)?;
    let base_obj_type = PackfileObjectType::from_str(base_obj_type.as_str())?;
    read_and_apply_delta_instructions(
        buffed_reader,
        len,
        base_obj_type,
        base_obj_len,
        base_obj_content,
        logger,
    )
}

fn read_ref_base_object(
    socket: &mut dyn Read,
    objects_data: Option<&HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>>,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<(String, usize, Vec<u8>), CommandError> {
    let mut base_obj_hash = [0; 20];
    socket
        .read_exact(&mut base_obj_hash)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    if let Some(objects_data) = objects_data {
        if let Some((base_obj_type, base_obj_len, base_obj_content)) =
            objects_data.get(&base_obj_hash)
        {
            return Ok((
                base_obj_type.to_string(),
                *base_obj_len,
                base_obj_content.to_vec(),
            ));
        }
    }
    let base_obj_hash_str = u8_vec_to_hex_string(&base_obj_hash);
    logger.log(&format!("Base object hash: {}", base_obj_hash_str));
    let (base_obj_type, base_obj_len, base_obj_content) =
        db.read_object_data(&base_obj_hash_str, logger)?;
    Ok((base_obj_type, base_obj_len, base_obj_content))
}

fn read_and_apply_delta_instructions(
    buffed_reader: &mut BuffedReader,
    len: usize,
    base_obj_type: PackfileObjectType,
    base_obj_len: usize,
    base_obj_content: Vec<u8>,
    logger: &mut Logger,
) -> Result<(PackfileObjectType, usize, Vec<u8>), CommandError> {
    let delta_content = read_extract_rewind(buffed_reader, len, logger)?;
    let mut cursor = Cursor::new(delta_content);
    let base_obj_len_redundant = read_size_encoding(&mut cursor)?;
    if base_obj_len_redundant != base_obj_len {
        return Err(CommandError::ErrorExtractingPackfileVerbose(format!(
            "Base object length ({}) is not equal to the one in the delta object ({})",
            base_obj_len, base_obj_len_redundant
        )));
    }
    let new_obj_expected_len = read_size_encoding(&mut cursor)?;
    let mut object_content = Vec::with_capacity(new_obj_expected_len);
    let mut writer_cursor = Cursor::new(&mut object_content);
    loop {
        let Some(instruction) = read_delta_instruction_from(&mut cursor, logger)? else {
            break;
        };
        instruction.apply(&mut writer_cursor, &base_obj_content)?;
    }
    if object_content.len() != new_obj_expected_len {
        return Err(CommandError::ErrorExtractingPackfileVerbose(format!(
            "Expected length: {}, Decompressed data length: {}",
            new_obj_expected_len,
            object_content.len(),
        )));
    }
    Ok((base_obj_type, new_obj_expected_len, object_content))
}

fn read_extract_rewind(
    buffed_reader: &mut BuffedReader<'_>,
    len: usize,
    logger: &mut Logger,
) -> Result<Vec<u8>, CommandError> {
    let base_post = buffed_reader.get_pos();
    let mut buffed_reader_ref: &mut BuffedReader<'_> = buffed_reader;
    if len == 0 {
        let mut null_comprresed = [0; 8];
        buffed_reader_ref
            .read_exact(&mut null_comprresed)
            .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
        if null_comprresed == [120, 156, 3, 0, 0, 0, 0, 1] {
            buffed_reader_ref.record_and_fast_foward_to(base_post + 8, logger)?;
            return Ok(Vec::new());
        }
        buffed_reader_ref.record_and_fast_foward_to(base_post, logger)?;
    }
    let mut decoder = flate2::read::ZlibDecoder::new(&mut buffed_reader_ref);
    let mut delta_content = vec![0; len]; // TODO with capacity
    decoder
        .read_exact(&mut delta_content)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let bytes_used = decoder.total_in() as usize;
    buffed_reader.record_and_fast_foward_to(base_post + bytes_used, logger)?;
    Ok(delta_content)
}

fn read_ofs(socket: &mut dyn Read) -> Result<u32, CommandError> {
    let mut offset = 0;
    loop {
        let mut byte = [0; 1];
        socket
            .read_exact(&mut byte)
            .map_err(|e| CommandError::VariableLengthEncodingOfs(e.to_string()))?;
        let byte_value = (byte[0] & 0b01111111) as u32;
        let last_byte = byte[0] & 0b10000000 == 0;

        offset = (offset << 7) | byte_value;
        if last_byte {
            return Ok(offset);
        }

        offset += 1;
    }
}

pub fn read_size_encoding(buffed_reader: &mut dyn Read) -> Result<usize, CommandError> {
    let mut bits = Vec::new();

    let mut is_last_byte: bool = false;
    while !is_last_byte {
        let mut seven_bit_chunk = Vec::<u8>::new();
        let mut current_byte_buf = [0; 1];
        buffed_reader
            .read_exact(&mut current_byte_buf)
            .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
        let current_byte = current_byte_buf[0];
        let seven_bit_chunk_with_zero = current_byte & 0b01111111;
        for i in (0..7).rev() {
            let bit = (seven_bit_chunk_with_zero >> i) & 1;
            seven_bit_chunk.push(bit);
        }
        bits.splice(0..0, seven_bit_chunk);
        is_last_byte = current_byte >> 7 == 0;
    }

    Ok(bits_to_usize(&bits))
}

pub fn read_object_header_from_packfile(
    buffed_reader: &mut BuffedReader,
) -> Result<(PackfileObjectType, usize), CommandError> {
    let mut first_byte_buf = [0; 1];
    buffed_reader
        .read_exact(&mut first_byte_buf)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
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
            .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
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
pub fn read_objects_from_packfile(
    socket: &mut TcpStream,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let mut buffed_reader = BuffedReader::new(socket);
    let object_number = read_packfile_header(&mut buffed_reader)?;
    read_objects_from_packfile_body(&mut buffed_reader, object_number, db, logger)
}

fn get_object_packfile_offset(
    sha1: &[u8; 20],
    index_file: &mut dyn Read,
    logger: &mut Logger,
) -> Result<Option<u32>, CommandError> {
    logger.log(&format!(
        "Searching object in packfile index: {}",
        u8_vec_to_hex_string(sha1)
    ));
    // The first four bytes are always 255, 116, 79, 99
    let mut header_bytes = [0; 4];
    index_file.read_exact(&mut header_bytes).map_err(|e| {
        CommandError::ErrorExtractingPackfileVerbose(format!(
            "Error al leer header del index: {}",
            e.to_string()
        ))
    })?;
    if header_bytes != [255, 116, 79, 99] {
        return Err(CommandError::ErrorExtractingPackfileVerbose(format!(
            "el error ocurre en la funcion get_object_packfile al 
        querer verificar el header {:?}",
            header_bytes
        )));
    }
    // The next four bytes denote the version number
    let mut version_bytes = [0; 4];
    index_file.read_exact(&mut version_bytes).map_err(|e| {
        CommandError::ErrorExtractingPackfileVerbose(format!(
            "Error al leer version del index: {}",
            e.to_string()
        ))
    })?;
    if version_bytes != [0, 0, 0, 2] {
        return Err(CommandError::ErrorExtractingPackfileVerbose(format!(
            "el error ocurre en la funcion get_object_packfile al 
        querer verificar el header {:?}",
            version_bytes
        )));
    }
    // # Fanout table
    // The first level of entries in this table is a series of 256 entries of four bytes
    // each, 1024 bytes long in total. According to the documentation, “[the] N-th entry
    // of this table records the number of objects in the corresponding pack, the first
    // byte of whose object name is less than or equal to N.”
    let (prev_object_group_count, object_count, number_of_objects_in_group) =
        read_first_layer(index_file, sha1)?;

    // The second layer of the fanout table contains the 20-byte object names, in order.
    // We already know how many to expect from the first layer of the fanout table.
    let Some(index_of_object) = read_seccond_layer(
        prev_object_group_count,
        index_file,
        number_of_objects_in_group,
        sha1,
        object_count,
    )?
    else {
        return Ok(None);
    };

    // The third layer of the fanout table gives us a four-byte cyclic redundancy check value for each object.
    let mut crcs = vec![0; object_count as usize * 4];
    index_file
        .read_exact(&mut crcs)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;

    // The fourth layer contains the information we’ve been looking for: the packfile offsets for each object.
    // These are also four bytes per entry.
    let mut prev_offsets = vec![0; index_of_object as usize * 4];
    index_file
        .read_exact(&mut prev_offsets)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let mut offset_bytes = vec![0; 4];
    index_file
        .read_exact(&mut offset_bytes)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let offset = u32::from_be_bytes(offset_bytes.clone().try_into().map_err(|_| {
        CommandError::ErrorExtractingPackfileVerbose(format!(
            "Could not get offset value: {:b} {:b} {:b} {:b}",
            offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3]
        ))
    })?);

    Ok(Some(offset))
}

fn read_seccond_layer(
    prev_object_group_count: u32,
    index_file: &mut dyn Read,
    number_of_objects_in_group: u32,
    sha1: &[u8; 20],
    object_count: u32,
) -> Result<Option<u32>, CommandError> {
    let mut prev_group_object_names = vec![0; prev_object_group_count as usize * 20];
    index_file
        .read_exact(&mut prev_group_object_names)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    // index_file
    //     .seek(SeekFrom::Current((prev_object_group_count * 20) as i64))
    //     .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let mut number_objets_read_in_group = 0;
    let mut found = false;
    for _ in 0..number_of_objects_in_group {
        let mut object_name = [0; 20];
        index_file
            .read_exact(&mut object_name)
            .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;

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
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    Ok(Some(index_of_object))
}

fn read_first_layer(
    index_file: &mut dyn Read,
    sha1: &[u8; 20],
) -> Result<(u32, u32, u32), CommandError> {
    let mut cumulative_objects_counts: Vec<u8> = vec![0; 1024];
    index_file
        .read_exact(&mut cumulative_objects_counts)
        .map_err(|e| CommandError::ErrorExtractingPackfileVerbose(e.to_string()))?;
    let object_group_index = sha1[0] as usize;
    let prev_object_group_count = if object_group_index == 0 {
        0
    } else {
        u32::from_be_bytes(
            cumulative_objects_counts[(object_group_index - 1) * 4..object_group_index * 4]
                .to_vec()
                .try_into()
                .map_err(|_| {
                    CommandError::ErrorExtractingPackfileVerbose(format!(
                        "Could not get prev_object_group_count value: {:?}, {}",
                        cumulative_objects_counts, object_group_index
                    ))
                })?,
        )
    };
    let object_group_count = u32::from_be_bytes(
        cumulative_objects_counts[object_group_index * 4..(object_group_index + 1) * 4]
            .to_vec()
            .try_into()
            .map_err(|_| {
                CommandError::ErrorExtractingPackfileVerbose(format!(
                    "Could not get object_group_count value: {:?}, {}",
                    cumulative_objects_counts, object_group_index
                ))
            })?,
    );
    let object_count = u32::from_be_bytes(
        cumulative_objects_counts[cumulative_objects_counts.len() - 4..]
            .to_vec()
            .try_into()
            .map_err(|_| {
                CommandError::ErrorExtractingPackfileVerbose(format!(
                    "Could not get object_count value: {:?}, {}",
                    cumulative_objects_counts, object_group_index
                ))
            })?,
    );
    let number_of_objects_in_group = object_group_count - prev_object_group_count;
    Ok((
        prev_object_group_count,
        object_count,
        number_of_objects_in_group,
    ))
}

pub fn search_object_data_from_hash(
    sha1: [u8; 20],
    mut index_file: &mut dyn Read,
    mut packfile: &mut dyn Read,
    db: &ObjectsDatabase,
    logger: &mut Logger,
) -> Result<Option<(PackfileObjectType, usize, Vec<u8>)>, CommandError> {
    let Some(packfile_offset) = get_object_packfile_offset(&sha1, &mut index_file, logger)? else {
        logger.log("Object not found in packfile");
        return Ok(None);
    };
    logger.log(&format!(
        "Object found in packfile index at offset: {}",
        packfile_offset
    ));
    let mut buffed_reader = BuffedReader::new(&mut packfile);
    Ok(Some(read_object_from_offset(
        &mut buffed_reader,
        packfile_offset,
        db,
        logger,
    )?))
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
        _packfile_bits: &Vec<u8>,
        hash: &str,
        expected_offset: u32,
    ) {
        let mut index_file = std::io::Cursor::new(index_file_bits);

        let sha1 = crate::utils::aux::hex_string_to_u8_vec(hash);
        let offset =
            get_object_packfile_offset(&sha1, &mut index_file, &mut Logger::new_dummy()).unwrap();
        assert_eq!(offset, Some(expected_offset));
    }

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "db963515880f98c8435df2ce902e4f337aac7ea6",
        12,
    );

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "df2b8fc99e1c1d4dbc0a854d9f72157f1d6ea078",
        143,
    );

    test_read_index(
        &index_file_bits,
        &packfile_bits,
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        186,
    );
}
