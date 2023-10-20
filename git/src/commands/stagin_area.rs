use std::{
    collections::HashMap,
    env,
    hash::Hash,
    io::{Read, Write},
};

use crate::logger::Logger;

use super::{
    branch_manager::get_last_commit,
    command_errors::CommandError,
    objects::{
        aux::{build_last_commit_tree, get_name_bis},
        git_object::GitObject,
        tree::Tree,
    },
    objects_database,
};

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

    pub fn get_changes(&self) -> Result<HashMap<String, String>, CommandError> {
        let tree_commit = build_last_commit_tree(&mut Logger::new_dummy())?;
        let mut changes: HashMap<String, String> = HashMap::new();
        if let Some(tree) = tree_commit {
            for (path, hash) in self.files.iter() {
                let (is_in_last_commit, name) = tree.has_blob_from_hash(hash)?;

                if !is_in_last_commit || get_name_bis(path)? != name {
                    _ = changes.insert(path.to_string(), hash.to_string())
                }
            }
        } else {
            changes = self.files.clone()
        }
        Ok(changes)
    }

    pub fn has_changes(&self) -> Result<bool, CommandError> {
        let changes = self.get_changes()?.len();
        let deleted_files = self.get_deleted_files()?;
        Ok(changes + deleted_files.len() > 0)
    }

    fn get_deleted_files(&self) -> Result<Vec<GitObject>, CommandError> {
        let mut deleted_blobs: Vec<GitObject> = Vec::new();
        let last_commit_tree = build_last_commit_tree(&mut Logger::new_dummy())?;
        if let Some(tree) = last_commit_tree {
            deleted_blobs = tree.get_deleted_blob(&self.files);
        }
        Ok(deleted_blobs)
    }

    pub fn has_file_from_path(&self, path: &str) -> bool {
        self.get_files().contains_key(path)
    }

    pub fn has_file_from_hash(&self, hash_obj: &str) -> bool {
        for (path, hash) in self.get_files() {
            if hash_obj == hash {
                return true;
            }
        }
        false
    }

    pub fn empty(&mut self, logger: &mut Logger) -> Result<(), CommandError> {
        let mut files = self.files.clone();
        let tree_commit = build_last_commit_tree(logger)?;
        if let Some(tree) = tree_commit {
            for (path, hash) in self.files.iter() {
                let name = get_name_bis(path)?;
                let (is_in_last_commit, name_found) = tree.has_blob_from_hash(hash)?;
                if !is_in_last_commit || name != name_found {
                    files.remove(path);
                }
            }
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

    pub(crate) fn write_tree(&self, logger: &mut Logger) -> Result<String, CommandError> {
        let working_tree = self.get_working_tree_staged(logger)?;

        let mut tree: GitObject = Box::new(working_tree);

        objects_database::write(logger, &mut tree)
    }

    pub fn get_working_tree_staged(&self, logger: &mut Logger) -> Result<Tree, CommandError> {
        let current_dir_display = "";
        let mut working_tree = Tree::new(current_dir_display.to_string());
        for (path, hash) in &self.files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            working_tree.add_path_tree(logger, vector_path, current_depth, hash)?;
        }
        Ok(working_tree)
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
