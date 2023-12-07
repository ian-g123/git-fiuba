fn get_objects_from_tree(
    hash_objects: &mut HashMap<String, GitObject>,
    tree: &Tree,
) -> Result<(), CommandError> {
    for (hash_object, mut git_object) in tree.get_objects() {
        if let Some(son_tree) = git_object.as_tree() {
            get_objects_from_tree(hash_objects, &son_tree)?;
        }
        hash_objects.insert(hash_object, git_object);
    }
    Ok(())
}

fn packfile_header(objects_number: u32) -> Vec<u8> {
    let mut header = Vec::<u8>::new();
    header.extend("PACK".as_bytes());
    header.extend(2u32.to_be_bytes());
    header.extend(objects_number.to_be_bytes());
    header
}

fn write_object_to_packfile(
    mut git_object: GitObject,
    packfile: &mut Vec<u8>,
) -> Result<(), CommandError> {
    let mut object_content = git_object.content(None)?;
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
        let Some(mut tree) = commit_object.get_tree() else {
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
