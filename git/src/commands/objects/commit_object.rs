use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use super::aux::get_sha1;
use super::git_object::{GitObject, GitObjectTrait};
use super::super_integers::{read_i32_from, read_i64_from, read_u32_from, SuperIntegers};
use super::super_string::{read_string_from, u8_vec_to_hex_string, SuperStrings};
use super::{author::Author, tree::Tree};
use crate::commands::command_errors::CommandError;
use crate::logger::Logger;

extern crate chrono;
use chrono::{prelude::*, DateTime};

#[derive(Clone)]
pub struct CommitObject {
    parents: Vec<String>,
    message: String,
    author: Author,
    committer: Author,
    timestamp: i64,
    offset: i32,
    tree: String,
}

impl CommitObject {
    /// Crea un objeto Commit.
    pub fn new(
        parent: Vec<String>,
        message: String,
        author: Author,
        committer: Author,
        timestamp: i64,
        offset: i32,
        tree: String,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            parents: parent,
            message,
            author,
            committer,
            timestamp,
            offset,
            tree,
        })
    }

    /// Devuelve el hash del tree del Commit.
    pub fn get_tree_hash(&self) -> String {
        self.tree.clone()
    }

    /// Crea un Commit a partir de la infromación leída del stream.
    pub fn read_from(
        stream: &mut dyn Read,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let (tree_hash, parents, author, author_timestamp, author_offset, committer, _, _, message) =
            read_commit_info_from(stream, logger)?;
        logger.log("commit created");
        Ok(Box::new(Self {
            tree: tree_hash,
            parents,
            author,
            committer,
            message,
            timestamp: author_timestamp,
            offset: author_offset,
        }))
    }

    /// Muestra la información del Commit, escribiéndola en el stream pasado.
    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        _: usize,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let (
            tree_hash,
            parents,
            author,
            author_timestamp,
            author_offset,
            committer,
            committer_timestamp,
            committer_offset,
            message,
        ) = read_commit_info_from(stream, logger)?;

        writeln!(output, "tree {}", tree_hash)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        for parent_hash in parents {
            writeln!(output, "parent {}", parent_hash)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }

        writeln!(
            output,
            "author {} {} {}",
            author,
            author_timestamp,
            offset_str(author_offset)?
        )
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        writeln!(
            output,
            "committer {} {} {}",
            committer,
            committer_timestamp,
            offset_str(committer_offset)?
        )
        .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        writeln!(output, "\n{}", message)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        Ok(())
    }
}

/// Lee la información de un Commit.
fn read_commit_info_from(
    stream: &mut dyn Read,
    logger: &mut Logger,
) -> Result<
    (
        String,
        Vec<String>,
        Author,
        i64,
        i32,
        Author,
        i64,
        i32,
        String,
    ),
    CommandError,
> {
    let tree_hash_be = read_hash_from(stream)?;
    let tree_hash = u8_vec_to_hex_string(&tree_hash_be);
    let number_of_parents = read_u32_from(stream)?;
    let parents = read_parents_from(number_of_parents, stream)?;
    let author = Author::read_from(stream)?;
    let author_timestamp = read_i64_from(stream)?;
    let author_offset = read_i32_from(stream)?;
    let committer = Author::read_from(stream)?;
    let committer_timestamp = read_i64_from(stream)?;
    let committer_offset = read_i32_from(stream)?;
    let message = read_string_from(stream)?;
    Ok((
        tree_hash,
        parents,
        author,
        author_timestamp,
        author_offset,
        committer,
        committer_timestamp,
        committer_offset,
        message,
    ))
}

/// Devuelve el offset en string.
fn offset_str(minutes: i32) -> Result<String, CommandError> {
    let sign = if minutes < 0 { "-" } else { "+" };
    let hours = minutes.abs() / 60;
    let minutes = minutes.abs() % 60;
    Ok(format!("{}{:02}{:02}", sign, hours, minutes))
}

/// Devuelve un vector con el hash de cada commit padre.
fn read_parents_from(
    number_of_parents: u32,
    stream: &mut dyn Read,
) -> Result<Vec<String>, CommandError> {
    let mut parents = Vec::<String>::new();
    for _ in 0..number_of_parents {
        let parent_hash_be = read_hash_from(stream)?;
        let parent_hash = u8_vec_to_hex_string(&parent_hash_be);
        parents.push(parent_hash);
    }
    Ok(parents)
}

/// Lee el hash del stream y lo devuelve en formato  Vec<u8>
fn read_hash_from(stream: &mut dyn Read) -> Result<[u8; 20], CommandError> {
    let mut tree_hash_be = [0; 20];
    stream
        .read_exact(&mut tree_hash_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    Ok(tree_hash_be)
}

impl GitObjectTrait for CommitObject {
    /// Devuelve la información de un commit.
    fn get_info_commit(&self) -> Option<(String, Author, Author, i64, i32)> {
        Some((
            self.message.clone(),
            self.author.clone(),
            self.committer.clone(),
            self.timestamp,
            self.offset,
        ))
    }

    fn as_commit(&self) -> Option<&CommitObject> {
        Some(self)
    }
    fn get_path(&self) -> Option<String> {
        None
    }
    fn type_str(&self) -> String {
        "commit".to_string()
    }

    fn mode(&self) -> super::mode::Mode {
        todo!()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&self.tree.cast_hex_to_u8_vec()?);
        let parents_len_be = (self.parents.len() as u32).to_be_bytes();
        buf.extend_from_slice(&parents_len_be);
        for parent in &self.parents {
            buf.extend_from_slice(&parent.cast_hex_to_u8_vec()?);
        }
        let mut stream = Cursor::new(&mut buf);
        stream
            .seek(SeekFrom::End(0))
            .map_err(|_| CommandError::FileWriteError("Error al mover el cursor".to_string()))?;
        self.author.write_to(&mut stream)?;
        self.timestamp.write_to(&mut stream)?;
        self.offset.write_to(&mut stream)?;
        // write_datetime_to(&mut stream, &self.date)?;

        self.committer.write_to(&mut stream)?;
        self.timestamp.write_to(&mut stream)?;
        self.offset.write_to(&mut stream)?;
        // write_datetime_to(&mut stream, &self.date)?;

        self.message.write_to(&mut stream)?;
        Ok(buf)
    }

    fn to_string_priv(&self) -> String {
        todo!()
    }

    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn get_hash(&mut self) -> Result<[u8; 20], CommandError> {
        Ok(get_sha1(&self.content()?))
    }
}

impl fmt::Display for CommitObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tree {}\nparent {:?}\nauthor {}\ncommitter {}\n\n{}",
            self.tree, self.parents, self.author, self.committer, self.message
        )
    }
}

/// Crea un DateTime<Local> a partir de la información recibida.
fn get_date(line: &mut Vec<&str>) -> Result<DateTime<Local>, CommandError> {
    let Some(time_zone_offset_str) = line.pop() else {
        return Err(CommandError::InvalidCommit);
    };
    let Some(timestamp_str) = line.pop() else {
        return Err(CommandError::InvalidCommit);
    };
    let offset_secconds = time_zone_offset_str.parse::<i32>().unwrap() * 3600;
    let time_stamp = timestamp_str.parse::<i64>().unwrap();
    let Some(offset) = chrono::FixedOffset::east_opt(offset_secconds) else {
        return Err(CommandError::InvalidCommit);
    };

    let Some(utc_datetime) = NaiveDateTime::from_timestamp_opt(time_stamp, 0) else {
        return Err(CommandError::InvalidCommit);
    };
    Ok(DateTime::<Local>::from_naive_utc_and_offset(
        utc_datetime,
        offset,
    ))
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Write};

    use crate::commands::{file_compressor::compress, objects::git_object};

    use super::*;

    fn write() -> Result<(), CommandError> {
        let Ok(mut file) = File::create(".git/objects/e3/540872766f87b1de467a5e867d656a6e6fe959")
        else {
            return Err(CommandError::CompressionError);
        };

        // Contenido que deseas escribir en el archivo
        let contenido = "100644 blob 09c857543fc52cd4267c3825644b4fd7f437dc3f .gitignore\n040000 tree d3a471637c78c8f67cca05221a942bd7efabb58c git".as_bytes();
        let contenido = compress(&contenido)?;

        // Escribe el contenido en el archivo
        if file.write_all(&contenido).is_err() {
            return Err(CommandError::CompressionError);
        }

        //

        let Ok(mut file) = File::create(".git/objects/d3/a471637c78c8f67cca05221a942bd7efabb58c")
        else {
            return Err(CommandError::CompressionError);
        };

        // Contenido que deseas escribir en el archivo
        let contenido = "100644 blob f0e37a3b70089bf8ead6970f2d4339527dc628a Cargo.lock\n100644 blob 5da01b81e6f2c1926d9e6df32dc160dfe5326239 Cargo.toml".as_bytes();
        let contenido = compress(&contenido)?;

        // Escribe el contenido en el archivo
        if file.write_all(&contenido).is_err() {
            return Err(CommandError::CompressionError);
        }
        Ok(())
    }

    // Write unit tests for write to and read from for commits:
    #[test]
    fn write_and_read() {
        // datetime for 1970-01-01 00:00:00 UTC
        let mut commit = CommitObject::new(
            vec![],
            "message".to_string(),
            Author::new("name", "email"),
            Author::new("name", "email"),
            1,
            -180,
            "a471637c78c8f67cca05221a942bd7efabb58caa".to_string(),
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut writer_stream = Cursor::new(&mut buf);
        commit.write_to(&mut writer_stream).unwrap();
        let mut reader_stream = Cursor::new(&mut buf);
        let mut fetched_commit = git_object::read_git_object_from(
            &mut reader_stream,
            "",
            "a471637c78c8f67cca05221a942bd7efabb58caa",
            &mut Logger::new_dummy(),
        )
        .unwrap();

        let mut fetched_commit_buf: Vec<u8> = Vec::new();
        let mut fetched_commit_writer_stream = Cursor::new(&mut fetched_commit_buf);
        fetched_commit
            .write_to(&mut fetched_commit_writer_stream)
            .unwrap();

        assert_eq!(buf, fetched_commit_buf);
    }

    // Write and display
    #[test]
    fn write_and_display() {
        let mut commit = CommitObject::new(
            vec![],
            "message".to_string(),
            Author::new("name", "email"),
            Author::new("name", "email"),
            1,
            -180,
            "a471637c78c8f67cca05221a942bd7efabb58caa".to_string(),
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut writer_stream = Cursor::new(&mut buf);
        commit.write_to(&mut writer_stream).unwrap();
        let mut output: Vec<u8> = Vec::new();
        let mut output_writer = Cursor::new(&mut output);
        let mut reader_stream = Cursor::new(&mut buf);
        git_object::display_from_stream(
            &mut reader_stream,
            &mut Logger::new_dummy(),
            &mut output_writer,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "tree a471637c78c8f67cca05221a942bd7efabb58caa\nauthor name <email> 1 -0300\ncommitter name <email> 1 -0300\n\nmessage\n".to_string());
    }
}
