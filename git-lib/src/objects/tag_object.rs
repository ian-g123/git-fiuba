use std::io::{Cursor, Read, Write};

use crate::{
    command_errors::CommandError,
    logger::Logger,
    utils::{aux::hex_string_to_u8_vec, super_string::SuperStrings},
};

use super::{
    author::Author,
    commit_object::{get_author_info, lines_next, offset_str},
    git_object::{self, GitObject, GitObjectTrait},
};

pub struct TagObject {
    name: String,
    object: String,
    object_type: String,
    message: String,
    tagger: Author,
    timestamp: i64,
    offset: i32,
    hash: [u8; 20],
}

impl TagObject {
    pub fn new(
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
            hash,
        }
    }
    pub fn read_from(
        stream: &mut dyn Read,
        logger: &mut Logger,
        hash_tag: String,
    ) -> Result<GitObject, CommandError> {
        let (object_hash, object_type, tag_name, tagger, timestamp, offset, message) =
            read_tag_info_from(stream)?;
        let hash = hash_tag.cast_hex_to_u8_vec()?;
        let tag = TagObject::new(
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
    let Some((_, object_type)) = object_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };

    let tag_name_line = lines_next(&mut lines)?;
    let Some((_, tag_name)) = object_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };

    let mut tagger_line = lines_next(&mut lines)?;

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

    fn content(
        &mut self,
        db: Option<&mut crate::objects_database::ObjectsDatabase>,
    ) -> Result<Vec<u8>, CommandError> {
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

    fn to_string_priv(&mut self) -> String {
        let Ok(content) = self.content(None) else {
            return "Error convirtiendo a utf8".to_string();
        };
        let Ok(string) = String::from_utf8(content.clone()) else {
            return "Error convirtiendo a utf8".to_string();
        };
        string
    }

    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        Ok(self.hash.clone())
    }

    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        None
    }

    fn get_path(&self) -> Option<String> {
        None
    }
}
