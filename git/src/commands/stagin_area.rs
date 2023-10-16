use std::{
    collections::HashMap,
    io::{Read, Write},
};

use super::command_errors::CommandError;

#[derive(Debug)]
pub struct StagingArea {
    pub files: HashMap<String, String>,
}

impl StagingArea {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
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
        staging_area.write_to(&mut file_writer_mock).unwrap();

        let mut file_reader_mock = Cursor::new(file_content_mock);
        let new_staging_area = StagingArea::read_from(&mut file_reader_mock).unwrap();

        assert_eq!(
            new_staging_area.files.get("test.txt").unwrap(),
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
            new_staging_area.files.get("test.txt").unwrap(),
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
            new_staging_area.files.get("test2.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c450"
        );
        assert_eq!(
            new_staging_area.files.get("test.txt").unwrap(),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }
}
