use std::{
    collections::HashMap,
    io::{Read, Write},
};

#[derive(Debug)]
pub struct StagingArea {
    pub files: HashMap<String, String>,
}

impl StagingArea {
    pub fn write_to(&self, stream: &mut dyn Write) -> std::io::Result<()> {
        for (path, hash) in &self.files {
            let size_be = (path.len() as u32).to_be_bytes();
            stream.write(&size_be)?;
            stream.write(path.as_bytes())?;
            stream.write(&hash.as_bytes())?;
        }
        Ok(())
    }

    pub fn read_from(stream: &mut dyn Read) -> std::io::Result<StagingArea> {
        let mut files = HashMap::new();
        loop {
            let mut size_be = [0; 4];
            match stream.read(&mut size_be) {
                Ok(0) => break,
                Ok(_) => (),
                Err(error) => return Err(error),
            }
            let size = u32::from_be_bytes(size_be) as usize;
            let mut path = vec![0; size];
            stream.read(&mut path)?;
            let mut hash = vec![0; 40];
            stream.read(&mut hash)?;
            files.insert(
                String::from_utf8(path).unwrap(),
                String::from_utf8(hash).unwrap(),
            );
        }
        Ok(Self { files })
    }

    pub fn open() -> std::io::Result<StagingArea> {
        let mut file = std::fs::File::open(".git/index")?;
        Self::read_from(&mut file)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut file = std::fs::File::create(".git/index")?;
        self.write_to(&mut file)
    }

    pub fn add(&mut self, path: &str, hash: &str) {
        self.files.insert(path.to_string(), hash.to_string());
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
        let mut file_writer_mock = Cursor::new(&mut file_content_mock);
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
    fn test_write_read_two_values() {
        let mut staging_area = StagingArea {
            files: HashMap::new(),
        };

        let mut file_content_mock: Vec<u8> = Vec::new();
        let mut file_writer_mock = Cursor::new(&mut file_content_mock);
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
