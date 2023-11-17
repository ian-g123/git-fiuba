use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::{
    join_paths,
    logger::Logger,
    objects::git_object::{GitObject, GitObjectTrait},
    objects_database::ObjectsDatabase,
    utils::{aux::get_name, super_string::SuperStrings},
};

use super::{command_errors::CommandError, objects::tree::Tree};

#[derive(Debug)]
pub struct StagingArea {
    files: HashMap<String, String>,
    unmerged_files: HashMap<String, (Option<String>, Option<String>, Option<String>)>,
    index_path: String,
}

impl StagingArea {
    fn new(index_path: &str) -> Self {
        Self {
            files: HashMap::new(),
            unmerged_files: HashMap::new(),
            index_path: index_path.to_string(),
        }
    }

    pub fn get_files(&self) -> HashMap<String, String> {
        self.files.clone()
    }

    pub fn get_hash_from_path(&self, path: &str) -> Result<String, CommandError> {
        match self.files.get(path) {
            Some(hash) => Ok(hash.to_string()),
            None => Err(CommandError::RmFromStagingAreaError(path.to_string())),
        }
    }

    pub fn is_in_staging_area(&self, path: &String) -> bool {
        return self.files.contains_key(path);
    }

    pub fn get_unmerged_files(
        &self,
    ) -> HashMap<String, (Option<String>, Option<String>, Option<String>)> {
        self.unmerged_files.clone()
    }

    pub fn get_changes(
        &self,
        last_commit_tree: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<HashMap<String, String>, CommandError> {
        let mut changes: HashMap<String, String> = HashMap::new();
        if let Some(mut tree) = last_commit_tree.clone() {
            for (path, hash) in self.files.iter() {
                let (is_in_last_commit, name) = tree.has_blob_from_hash(hash, logger)?;

                if !is_in_last_commit || get_name(path)? != name {
                    _ = changes.insert(path.to_string(), hash.to_string())
                }
            }
        } else {
            changes = self.files.clone()
        }
        Ok(changes)
    }

    pub fn remove_from_stagin_area(
        &mut self,
        path: &str,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if !self.is_in_staging_area(&path.to_string()) {
            return Err(CommandError::RmFromStagingAreaError(path.to_string()));
        }

        self.remove(path);

        logger.log(&format!("staging_area.rm({})", path));
        Ok(())
    }

    pub fn has_changes(
        &self,
        _db: &ObjectsDatabase,
        last_commit_tree: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<bool, CommandError> {
        let changes = self.get_changes(last_commit_tree, logger)?.len();
        let deleted_files = self.get_deleted_files(last_commit_tree);
        Ok(changes + deleted_files.len() > 0)
    }

    pub fn get_deleted_files(&self, last_commit_tree: &Option<Tree>) -> Vec<String> {
        let mut deleted: Vec<String> = Vec::new();
        if let Some(mut tree) = last_commit_tree.clone() {
            self.check_deleted_from_commit(&mut tree, &mut deleted, "".to_string())
        }
        deleted
    }

    fn check_deleted_from_commit(&self, tree: &mut Tree, deleted: &mut Vec<String>, path: String) {
        for (name, (_, object_opt)) in tree.get_objects().iter_mut() {
            let complete_path = {
                if path == "".to_string() {
                    format!("{}", name)
                } else {
                    format!("{}/{}", path, name)
                }
            };
            let Some(object) = object_opt else {
                continue;
            };
            if let Some(new_tree) = object.as_mut_tree() {
                self.check_deleted_from_commit(new_tree, deleted, complete_path);
            } else {
                if !self.has_file_from_path(&complete_path) {
                    _ = deleted.push(complete_path);
                }
            }
        }
    }

    pub fn has_file_from_path(&self, path: &str) -> bool {
        self.get_files().contains_key(path) || self.unmerged_files.contains_key(path)
    }

    /// Verifica si hay un cambio del working tree respecto del staging area
    pub fn has_file_renamed(&self, actual_path: &str, actual_hash: &str) -> bool {
        for (path, hash) in self.get_files() {
            let mut actual_parts: Vec<&str> = actual_path.split_terminator("/").collect();
            let mut parts: Vec<&str> = path.split_terminator("/").collect();

            _ = actual_parts.pop();
            _ = parts.pop();

            if actual_parts == parts && hash == actual_hash {
                return true;
            }
        }
        false
    }

    pub fn has_file_from_hash(&self, hash_obj: &str) -> bool {
        for (_, hash) in self.get_files() {
            if hash_obj == hash {
                return true;
            }
        }
        false
    }

    pub fn remove_changes(
        &mut self,
        tree_commit: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut files = self.files.clone();

        if let Some(mut tree) = tree_commit.clone() {
            for (path, hash) in self.files.iter() {
                let (is_in_last_commit, name) = tree.has_blob_from_hash(hash, logger)?;
                if !is_in_last_commit || (name != get_name(&path)?) {
                    logger.log(&format!("Eliminando del staging area: {}", path));
                    files.remove(path);
                }
            }
        } else {
            files = HashMap::new();
        }
        self.files = files;
        Ok(())
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        for (path, hash) in &self.files {
            path.write_to(stream)?;
            stream
                .write(&[0])
                .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
            write_hash_str_to(stream, hash)?;
        }
        for (path, (common_hash, head_hash, destin_hash)) in &self.unmerged_files {
            path.write_to(stream)?;
            stream
                .write(&[1])
                .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
            self.write_hash(stream, common_hash)?;
            self.write_hash(stream, head_hash)?;
            self.write_hash(stream, destin_hash)?;
        }
        Ok(())
    }

    fn write_hash(
        &self,
        stream: &mut dyn Write,
        hash: &Option<String>,
    ) -> Result<(), CommandError> {
        match hash {
            Some(hash) => {
                stream
                    .write(&[1])
                    .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;

                write_hash_str_to(stream, hash)?;
            }
            None => {
                stream
                    .write(&[0])
                    .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn read_from(stream: &mut dyn Read, index_path: &str) -> Result<StagingArea, CommandError> {
        let mut files = HashMap::<String, String>::new();
        let mut unmerged_files =
            HashMap::<String, (Option<String>, Option<String>, Option<String>)>::new();
        loop {
            let mut size_be = [0; 4];
            match stream.read(&mut size_be) {
                Ok(0) => break,
                Ok(_) => (),
                Err(error) => return Err(CommandError::FailToOpenStaginArea(error.to_string())),
            }
            let size = u32::from_be_bytes(size_be) as usize;
            let mut path_be = vec![0; size];
            stream
                .read(&mut path_be)
                .map_err(|error| CommandError::FailToOpenStaginArea(error.to_string()))?;
            let path = String::from_utf8(path_be).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error convirtiendo path a string{}",
                    error.to_string()
                ))
            })?;
            let is_conflict_byte = {
                let mut is_conflict_byte = [0; 1];
                stream
                    .read_exact(&mut is_conflict_byte)
                    .map_err(|error| CommandError::FileReadError(error.to_string()))?;
                is_conflict_byte[0]
            };
            if is_conflict_byte == 0 {
                let hash = read_hash_str_from(stream)?;
                _ = files.insert(path, hash);
            } else {
                let common_hash = Self::read_hash(stream)?;
                let head_hash = Self::read_hash(stream)?;
                let destin_hash = Self::read_hash(stream)?;

                _ = unmerged_files.insert(path, (common_hash, head_hash, destin_hash));
            }
        }
        Ok(Self {
            files,
            unmerged_files,
            index_path: index_path.to_string(),
        })
    }

    fn read_hash(stream: &mut dyn Read) -> Result<Option<String>, CommandError> {
        let mut is_some_byte = [0; 1];
        stream
            .read_exact(&mut is_some_byte)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;

        let hash = if is_some_byte[0] == 0 {
            None
        } else {
            let hash = read_hash_str_from(stream)?;
            Some(hash)
        };
        Ok(hash)
    }

    pub fn open(git_path: &str) -> Result<StagingArea, CommandError> {
        let index_path = join_paths!(git_path, "index").ok_or(
            CommandError::FailToOpenStaginArea("No se pudo abrir el archivo index".to_string()),
        )?;
        match std::fs::File::open(&index_path) {
            Err(_) => Ok(StagingArea::new(&index_path)),
            Ok(mut file) => StagingArea::read_from(&mut file, &index_path),
        }
    }

    pub fn save(&self) -> Result<(), CommandError> {
        match std::fs::File::create(&self.index_path) {
            Err(error) => Err(CommandError::FailToSaveStaginArea(error.to_string())),
            Ok(mut file) => self.write_to(&mut file),
        }
    }

    pub fn add(&mut self, path: &str, hash: &str) {
        let key: String = if path.to_string().starts_with("./") {
            path[2..].to_string()
        } else {
            path.to_string()
        };
        if self.unmerged_files.contains_key(path) {
            _ = self.unmerged_files.remove(path);
        }
        self.files.insert(key, hash.to_string());
    }

    pub fn add_unmerged_file(
        &mut self,
        path: &str,
        common_hash: Option<String>,
        head_hash: Option<String>,
        destin_hash: Option<String>,
    ) {
        let key: String = if path.to_string().starts_with("./") {
            path[2..].to_string()
        } else {
            path.to_string()
        };
        if self.files.contains_key(path) {
            _ = self.files.remove(path);
        }
        self.unmerged_files
            .insert(key, (common_hash, head_hash, destin_hash));
    }

    pub fn remove(&mut self, path: &str) {
        self.files.remove(path);
    }

    // pub fn write_tree(&mut self, logger: &mut Logger) -> Result<String, CommandError> {
    //     let working_tree = self.get_working_tree_staged(logger)?;

    //     let mut tree: GitObject = Box::new(working_tree);

    //     objects_database::write(logger, &mut tree)
    // }

    pub fn get_working_tree_staged(&mut self, logger: &mut Logger) -> Result<Tree, CommandError> {
        let current_dir_display = "";
        let mut working_tree = Tree::new(current_dir_display.to_string());
        let files = self.sort_files();
        for (path, hash) in files.iter() {
            logger.log(&format!("path: {}", path));
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            working_tree.add_path_tree(logger, vector_path, current_depth, hash)?;
        }
        logger.log("Working tree staged built");
        Ok(working_tree)
    }

    pub fn get_working_tree_staged_bis(
        &mut self,
        last_commit_tree: &Option<Tree>,
        logger: &mut Logger,
        new_files: Vec<String>,
    ) -> Result<Tree, CommandError> {
        let current_dir_display = "";
        let mut working_tree = Tree::new(current_dir_display.to_string());
        let files = self.sort_files();
        for (path, hash) in files.iter() {
            let is_in_last_commit = {
                if let Some(mut tree) = last_commit_tree.clone() {
                    let (has_hash, name) = tree.has_blob_from_hash(hash, logger)?;

                    has_hash && get_name(path)? == name
                } else {
                    false
                }
            };

            if new_files.contains(path) || is_in_last_commit {
                let vector_path = path.split("/").collect::<Vec<_>>();
                let current_depth: usize = 0;
                working_tree.add_path_tree(logger, vector_path, current_depth, hash)?;
            }
        }
        Ok(working_tree)
    }

    fn sort_files(&mut self) -> Vec<(String, String)> {
        let mut keys: Vec<&String> = self.files.keys().collect();
        keys.sort();

        let mut sorted_files: Vec<(String, String)> = Vec::new();
        for key in keys {
            if let Some(value) = self.files.get(key) {
                sorted_files.push((key.clone(), value.clone()));
            }
        }
        sorted_files
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.unmerged_files.clear();
    }

    pub fn update_to_conflictings(
        &mut self,
        merged_files: HashMap<String, String>,
        unmerged_files: HashMap<String, (Option<String>, Option<String>, Option<String>)>,
    ) {
        self.files = merged_files;
        self.unmerged_files = unmerged_files;
    }

    pub fn update_to_tree(&mut self, working_dir: &Tree) -> Result<(), CommandError> {
        self.files.clear();
        let mut boxed_working_dir: GitObject = Box::new(working_dir.to_owned());
        self.add_object(&mut boxed_working_dir, "")
    }

    pub fn add_object(
        &mut self,
        object: &mut GitObject,
        obj_path: &str,
    ) -> Result<(), CommandError> {
        if let Some(blob) = object.as_mut_blob() {
            self.add(&obj_path, &blob.get_hash_string()?);
        } else if let Some(tree) = object.as_mut_tree() {
            for (name, (_, child_obj_opt)) in tree.get_objects() {
                let Some(mut child_object) = child_obj_opt else {
                    return Err(CommandError::ShallowTree);
                };
                let path = join_paths!(obj_path, name).ok_or(
                    CommandError::FailToSaveStaginArea("Fail to join paths".to_string()),
                )?;
                self.add_object(&mut child_object, &path)?;
            }
        }
        Ok(())
    }

    pub fn add_unmerged_object(
        &mut self,
        original_object: &mut GitObject,
        modiffied_object: &mut GitObject,
        obj_path: &str,
        modiffied_object_is_head: bool,
    ) -> Result<(), CommandError> {
        if let Some(modiffied_object_blob) = modiffied_object.as_mut_blob() {
            let Some(original_object_blob) = original_object.as_mut_blob() else {
                return Err(CommandError::CannotHaveFileAndFolderWithSameName(
                    obj_path.to_string(),
                ));
            };
            if modiffied_object_is_head {
                self.add_unmerged_file(
                    &obj_path,
                    Some(original_object_blob.get_hash_string()?),
                    Some(modiffied_object_blob.get_hash_string()?),
                    None,
                );
            } else {
                self.add_unmerged_file(
                    &obj_path,
                    Some(original_object.get_hash_string()?),
                    None,
                    Some(modiffied_object.get_hash_string()?),
                );
            }
        } else if let Some(modiffied_object_tree) = modiffied_object.as_mut_tree() {
            let Some(original_object_tree) = original_object.as_mut_tree() else {
                return Err(CommandError::CannotHaveFileAndFolderWithSameName(
                    obj_path.to_string(),
                ));
            };
            for (name, (_, modiffied_object_child_opt)) in modiffied_object_tree.get_objects() {
                let Some(mut modiffied_object_child) = modiffied_object_child_opt else {
                    return Err(CommandError::ShallowTree);
                };
                let path = join_paths!(obj_path, name).ok_or(
                    CommandError::FailToSaveStaginArea("Fail to join paths".to_string()),
                )?;
                let (_, original_obj_opt) = original_object_tree
                    .get_objects()
                    .get(&name)
                    .ok_or(CommandError::CannotHaveFileAndFolderWithSameName(
                        obj_path.to_string(),
                    ))?
                    .to_owned();
                let Some(mut original) = original_obj_opt else {
                    return Err(CommandError::ShallowTree);
                };
                self.add_unmerged_object(
                    &mut original,
                    &mut modiffied_object_child,
                    &path,
                    modiffied_object_is_head,
                )?;
            }
        }
        Ok(())
    }

    pub fn has_conflicts(&self) -> bool {
        !self.unmerged_files.is_empty()
    }
}

fn write_hash_str_to(stream: &mut dyn Write, hash: &String) -> Result<(), CommandError> {
    stream
        .write(&hash.as_bytes())
        .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
    Ok(())
}

fn read_hash_str_from(stream: &mut dyn Read) -> Result<String, CommandError> {
    let mut hash_be = vec![0; 40];
    stream
        .read(&mut hash_be)
        .map_err(|error| CommandError::FailToOpenStaginArea(error.to_string()))?;
    let hash = String::from_utf8(hash_be).map_err(|error| {
        CommandError::FileReadError(format!(
            "Error convirtiendo hash a string{}",
            error.to_string()
        ))
    })?;
    Ok(hash)
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_write_read() {
        let mut staging_area = StagingArea {
            files: HashMap::new(),
            unmerged_files: HashMap::new(),

            index_path: "".to_string(),
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        println!("files: {:?}", staging_area.get_files());
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock, "").unwrap();
        println!("files: {:?}", new_staging_area.get_files());

        assert_eq!(
            new_staging_area.get_files().get("test.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }

    #[test]
    fn test_write_read_2() {
        let mut staging_area = StagingArea {
            files: HashMap::new(),
            unmerged_files: HashMap::new(),
            index_path: "".to_string(),
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("./test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock, "").unwrap();

        assert_eq!(
            new_staging_area.get_files().get("test.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }

    #[test]
    fn test_write_read_two_values() {
        let mut staging_area = StagingArea {
            files: HashMap::new(),
            unmerged_files: HashMap::new(),
            index_path: "".to_string(),
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        staging_area.add("test2.txt", "30d74d258442c7c65512eafab474568dd706c450");
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock, "").unwrap();

        assert_eq!(
            new_staging_area.get_files().get("test2.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c450"
        );
        assert_eq!(
            new_staging_area.get_files().get("test.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }
}
