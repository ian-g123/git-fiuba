use super::{
    command_errors::CommandError,
    objects::{tree::Tree, tree_or_blob::GitObject},
};

pub(crate) fn write_object(git_object: GitObject) -> Result<String, CommandError> {
    // let header = self.get_header(&content);
    let mut data = Vec::new();

    // data.extend_from_slice(header.as_bytes());
    // data.extend_from_slice(&content);
    git_object.write_to(&mut data)?;

    let parent_path = format!(".git/objects/{}", folder_name);
    let folder_name = &hex_string[0..2];
    let file_name = &hex_string[2..];
    let path = format!("{}/{}", parent_path, file_name);

    // let header = self.get_header(&content);
    // let mut data = Vec::new();

    // data.extend_from_slice(header.as_bytes());
    // data.extend_from_slice(&content);

    // let hex_string = self.get_sha1(&data);
    // if self.write {
    //     let folder_name = &hex_string[0..2];
    //     let file_name = &hex_string[2..];

    //     let parent_path = format!(".git/objects/{}", folder_name);
    //     let path = format!("{}/{}", parent_path, file_name);
    //     if let Err(error) = fs::create_dir_all(parent_path) {
    //         return Err(CommandError::FileOpenError(error.to_string()));
    //     };
    //     let Ok(mut file) = File::create(&path) else {
    //         return Err(CommandError::FileOpenError(
    //             "Error al abrir archivo para escritura".to_string(),
    //         ));
    //     };
    //     let compressed_data = compress(&data)?;
    //     if let Err(error) = file.write_all(&compressed_data) {
    //         return Err(CommandError::FileWriteError(error.to_string()));
    //     };
    //     return Ok((hex_string, Some(path)));
    // }
    // Ok((hex_string, None))
}
