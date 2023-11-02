use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::{
    logger::Logger, objects::git_object::GitObjectTrait, objects_database::ObjectsDatabase,
    utils::aux::get_name,
};

use super::{command_errors::CommandError, objects::tree::Tree};

#[derive(Debug)]
pub struct StagingArea {
    files: HashMap<String, String>,
}

impl StagingArea {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn get_files(&self) -> HashMap<String, String> {
        self.files.clone()
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

    pub fn has_changes(
        &self,
        db: &ObjectsDatabase,
        last_commit_tree: &Option<Tree>,
        logger: &mut Logger,
    ) -> Result<bool, CommandError> {
        let changes = self.get_changes(last_commit_tree, logger)?.len();
        let deleted_files = self.get_deleted_files(db, last_commit_tree)?;
        Ok(changes + deleted_files.len() > 0)
    }

    pub fn get_deleted_files(
        &self,
        db: &ObjectsDatabase,
        last_commit_tree: &Option<Tree>,
    ) -> Result<Vec<String>, CommandError> {
        let mut deleted: Vec<String> = Vec::new();
        if let Some(mut tree) = last_commit_tree.clone() {
            self.check_deleted_from_commit(&mut tree, &mut deleted, "".to_string())
        }
        Ok(deleted)
    }

    fn check_deleted_from_commit(&self, tree: &mut Tree, deleted: &mut Vec<String>, path: String) {
        for (name, object) in tree.get_objects().iter_mut() {
            let complete_path = {
                if path == "".to_string() {
                    format!("{}", name)
                } else {
                    format!("{}/{}", path, name)
                }
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
        self.get_files().contains_key(path)
    }

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
            let size_be = (path.len() as u32).to_be_bytes();
            stream
                .write(&size_be)
                .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
            stream
                .write(path.as_bytes())
                .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
            stream
                .write(&hash.as_bytes())
                .map_err(|error| CommandError::FailToSaveStaginArea(error.to_string()))?;
        }
        Ok(())
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<StagingArea, CommandError> {
        let mut files = HashMap::<String, String>::new();
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
            let mut hash_be = vec![0; 40];
            stream
                .read(&mut hash_be)
                .map_err(|error| CommandError::FailToOpenStaginArea(error.to_string()))?;
            let path = String::from_utf8(path_be).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error convirtiendo path a string{}",
                    error.to_string()
                ))
            })?;
            let hash = String::from_utf8(hash_be).map_err(|error| {
                CommandError::FileReadError(format!(
                    "Error convirtiendo hash a string{}",
                    error.to_string()
                ))
            })?;
            files.insert(path, hash);
        }
        Ok(Self { files })
    }

    pub fn open() -> Result<StagingArea, CommandError> {
        match std::fs::File::open(".git/index") {
            Err(_) => Ok(StagingArea::new()),
            Ok(mut file) => StagingArea::read_from(&mut file),
        }
    }

    pub fn save(&self) -> Result<(), CommandError> {
        match std::fs::File::create(".git/index") {
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
        self.files.insert(key, hash.to_string());
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
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            working_tree.add_path_tree(logger, vector_path, current_depth, hash)?;
        }
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
                    logger.log(&format!(
                        "Last commit tree : {}",
                        String::from_utf8_lossy(&tree.content(None)?)
                    ));
                    let (has_hash, name) = tree.has_blob_from_hash(hash, logger)?;
                    logger.log(&format!(
                        "Has hash: {}, name_in_commit: {}, name: {}",
                        has_hash,
                        name,
                        get_name(path)?
                    ));
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
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        println!("files: {:?}", staging_area.get_files());
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock).unwrap();
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
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("./test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock).unwrap();

        assert_eq!(
            new_staging_area.get_files().get("test.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }

    #[test]
    fn test_write_read_two_values() {
        let mut staging_area = StagingArea {
            files: HashMap::new(),
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock); // probar no crear dos mocks

        staging_area.add("test.txt", "30d74d258442c7c65512eafab474568dd706c430");
        staging_area.add("test2.txt", "30d74d258442c7c65512eafab474568dd706c450");
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock).unwrap();

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
