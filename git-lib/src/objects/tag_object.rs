use std::io::{Cursor, Read, Write};

use crate::{
    command_errors::CommandError,
    logger::Logger,
    objects_database::ObjectsDatabase,
    utils::{aux::get_sha1, super_string::SuperStrings},
};

use super::{
    author::Author,
    commit_object::{get_author_info, lines_next, offset_str},
    git_object::{GitObject, GitObjectTrait},
};

#[derive(PartialEq, Debug, Clone)]
pub struct TagObject {
    name: String,
    object: String,
    object_type: String,
    message: String,
    tagger: Author,
    timestamp: i64,
    offset: i32,
    hash: Option<[u8; 20]>,
}

impl TagObject {
    /// Crea un TagObject con su hash.
    pub fn new_from_hash(
        name: String,
        object: String,
        object_type: String,
        message: String,
        tagger: Author,
        timestamp: i64,
        offset: i32,
        hash: [u8; 20],
    ) -> TagObject {
        Self {
            name,
            object,
            object_type,
            message,
            tagger,
            timestamp,
            offset,
            hash: Some(hash),
        }
    }

    /// Crea un TagObject sin su hash.
    pub fn new(
        name: String,
        object: String,
        object_type: String,
        message: String,
        tagger: Author,
        timestamp: i64,
        offset: i32,
    ) -> TagObject {
        Self {
            name,
            object,
            object_type,
            message,
            tagger,
            timestamp,
            offset,
            hash: None,
        }
    }

    /// Crea un GitObject de tipo Tag a partir de su contenido.
    pub fn read_from(
        stream: &mut dyn Read,
        logger: &mut Logger,
        hash_tag: String,
    ) -> Result<GitObject, CommandError> {
        let (object_hash, object_type, tag_name, tagger, timestamp, offset, message) =
            read_tag_info_from(stream)?;
        let hash = hash_tag.cast_hex_to_u8_vec()?;
        let tag = TagObject::new_from_hash(
            tag_name,
            object_hash,
            object_type,
            message,
            tagger,
            timestamp,
            offset,
            hash,
        );
        logger.log("tag created");
        Ok(Box::new(tag))
    }

    /// Muestra información del tag a partir de su contenido.
    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        _: usize,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        let (object_hash, object_type, tag_name, tagger, timestamp, offset, message) =
            read_tag_info_from(stream)?;
        writeln!(output, "object {}", object_hash)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        writeln!(output, "type {}", object_type)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        writeln!(output, "name {}", tag_name)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        writeln!(
            output,
            "tagger {} {} {}",
            tagger,
            timestamp,
            offset_str(offset)?
        )
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        writeln!(output, "\n{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }

    /// Guarda el hash del tag.
    fn set_hash(&mut self, hash: [u8; 20]) {
        self.hash = Some(hash);
    }

    pub fn get_object_hash(&self) -> String {
        self.object.clone()
    }
}

fn read_tag_info_from(
    stream: &mut dyn Read,
) -> Result<(String, String, String, Author, i64, i32, String), CommandError> {
    let mut content = String::new();
    stream
        .read_to_string(&mut content)
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;

    let mut lines = content.lines();
    let object_line = lines_next(&mut lines)?;
    let Some((_, object_hash_str)) = object_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };

    let object_type_line = lines_next(&mut lines)?;
    let Some((_, object_type)) = object_type_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };

    let tag_name_line = lines_next(&mut lines)?;
    let Some((_, tag_name)) = tag_name_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };

    let tagger_line = lines_next(&mut lines)?;

    let Some((_, tagger_info)) = tagger_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };
    let (tagger, tagger_timestamp, tagger_offset) = get_author_info(tagger_info)?;

    lines_next(&mut lines)?;
    let message = lines.collect();

    Ok((
        object_hash_str.to_owned(),
        object_type.to_owned(),
        tag_name.to_owned(),
        tagger,
        tagger_timestamp,
        tagger_offset,
        message,
    ))
}

impl GitObjectTrait for TagObject {
    fn as_mut_tag(&mut self) -> Option<&mut TagObject> {
        Some(self)
    }

    fn as_tag(&mut self) -> Option<TagObject> {
        Some(self.to_owned())
    }

    fn as_mut_tree(&mut self) -> Option<&mut super::tree::Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        todo!()
    }

    fn type_str(&self) -> String {
        "tag".to_string()
    }

    fn mode(&self) -> super::mode::Mode {
        todo!()
    }

    fn content(&mut self, _db: Option<&ObjectsDatabase>) -> Result<Vec<u8>, CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(&mut buf);

        writeln!(stream, "object {}", self.object)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        writeln!(stream, "type {}", self.object_type)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        writeln!(stream, "tag {}", self.name)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        write!(stream, "tagger ").map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        self.tagger.write_to(&mut stream)?;

        let offset_hr = self.offset / 60;
        let offset_min = self.offset % 60;
        let offset_hr_str = {
            if offset_hr < 0 {
                format!("-{:02}", offset_hr.abs())
            } else {
                format!("{:02}", offset_hr)
            }
        };
        writeln!(
            stream,
            "{} {}{:02}",
            self.timestamp, offset_hr_str, offset_min
        )
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        writeln!(stream, "\n{}", self.message).map_err(|_| {
            CommandError::FileWriteError("Error al escribir el mensaje".to_string())
        })?;

        Ok(buf)
    }

    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        if let Some(hash) = self.hash {
            return Ok(hash);
        }
        let mut buf: Vec<u8> = Vec::new();
        self.write_to(&mut buf, None)?;
        let hash = get_sha1(&buf);
        self.set_hash(hash);
        Ok(hash)
    }

    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        None
    }

    fn get_path(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod test {

    use std::io::{Seek, SeekFrom};

    use super::*;
    use crate::{objects::git_object::read_git_object_from, objects_database::ObjectsDatabase};

    #[test]
    fn test_read_and_write() {
        let hash = "9bba3e612249063ee15b2cf537d303ef33b2032d"
            .to_string()
            .cast_hex_to_u8_vec()
            .unwrap();
        let mut tag = TagObject::new_from_hash(
            "tag1".to_string(),
            "754f91b7ebd0c7c2d0c962aaac5e96e2548d6e34".to_string(),
            "commit".to_string(),
            "tag1".to_string(),
            Author::new("name", "email"),
            1700314430,
            -0300,
            hash,
        );

        let mut buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(&mut buf);
        tag.write_to(&mut stream, None).unwrap();

        stream.seek(SeekFrom::Start(0)).unwrap();

        let mut tag2 = read_git_object_from(
            &ObjectsDatabase::new("").unwrap(),
            &mut stream,
            "",
            "9bba3e612249063ee15b2cf537d303ef33b2032d",
            &mut Logger::new_dummy(),
        )
        .unwrap();
        let tag2 = tag2.as_mut_tag().unwrap();
        assert_eq!(&mut tag, tag2);
    }

    #[test]
    fn test_get_hash() {
        let mut tag = TagObject::new(
            "tag1".to_string(),
            "754f91b7ebd0c7c2d0c962aaac5e96e2548d6e34".to_string(),
            "commit".to_string(),
            "tag1".to_string(),
            Author::new("Sofia-gb", "sofiagomezb@yahoo.com.ar"),
            1700314430,
            -180,
        );

        let expected = "9bba3e612249063ee15b2cf537d303ef33b2032d";
        let result = tag.get_hash_string().unwrap();
        assert_eq!(result, expected);
    }
}
