use super::git_object::{GitObject, GitObjectTrait};
use super::{author::Author, tree::Tree};
use crate::command_errors::CommandError;
use crate::logger::Logger;
use crate::objects_database::ObjectsDatabase;
use crate::utils::aux::{get_sha1, hex_string_to_u8_vec, read_string_until};
use crate::utils::super_string::u8_vec_to_hex_string;
use std::io::{Cursor, Read, Write};

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
    tree: Tree,
    hash: Option<[u8; 20]>,
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
        tree: Tree,
        hash: Option<[u8; 20]>,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            parents: parent,
            message,
            author,
            committer,
            timestamp,
            offset,
            tree,
            hash,
        })
    }
    pub fn get_parents(&self) -> Vec<String> {
        self.parents.clone()
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Devuelve el hash del tree del Commit.
    pub fn get_tree_hash(&mut self) -> Result<String, CommandError> {
        Ok(u8_vec_to_hex_string(&self.tree.get_hash()?))
    }

    /// Crea un Commit a partir de la infromación leída del stream.
    pub fn read_from(
        db: &ObjectsDatabase,
        stream: &mut dyn Read,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let (tree_hash, parents, author, author_timestamp, author_offset, committer, _, _, message) =
            read_commit_info_from(stream)?;
        let tree_hash_str = u8_vec_to_hex_string(&tree_hash);
        logger.log(&format!(
            "Reading tree hash from database: {}",
            tree_hash_str
        ));
        let mut tree = db.read_object(&tree_hash_str, logger)?;
        logger.log(&format!(
            "tree content en read_from : {}",
            String::from_utf8_lossy(&(tree.to_owned().content(None)?))
        ));
        let Some(tree) = tree.as_tree() else {
            return Err(CommandError::InvalidCommit);
        };
        Ok(Box::new(Self {
            tree,
            parents,
            author,
            committer,
            message,
            timestamp: author_timestamp,
            offset: author_offset,
            hash: None,
        }))
    }

    /// Muestra la información del Commit, escribiéndola en el stream pasado.
    pub(crate) fn display_from_stream(
        stream: &mut dyn Read,
        _: usize,
        output: &mut dyn Write,
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
        ) = read_commit_info_from(stream)?;
        let tree_hash_str = u8_vec_to_hex_string(&tree_hash);
        writeln!(output, "tree {}", tree_hash_str)
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

    pub fn get_tree(&self) -> &Tree {
        &self.tree
    }
}

fn read_commit_info_from(
    stream: &mut dyn Read,
) -> Result<
    (
        [u8; 20],
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
    let mut content = String::new();
    stream
        .read_to_string(&mut content)
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;

    let mut lines = content.lines();
    let tree_line = lines_next(&mut lines)?;
    let Some((_, tree_hash_str)) = tree_line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };
    let tree_hash_be = hex_string_to_u8_vec(tree_hash_str);
    let mut line = lines_next(&mut lines)?;
    let mut parents = Vec::new();
    loop {
        let Some((word, hash)) = line.split_once(' ') else {
            return Err(CommandError::InvalidCommit);
        };
        if word != "parent" {
            break;
        }
        parents.push(hash.to_string());
        let line_temp = lines_next(&mut lines)?;
        line = line_temp;
    }

    let Some((_, author_info)) = line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };
    let (author, author_timestamp, author_offset) = get_author_info(author_info)?;

    let line = lines_next(&mut lines)?;
    let Some((_, commiter_info)) = line.split_once(' ') else {
        return Err(CommandError::InvalidCommit);
    };
    let (committer, committer_timestamp, committer_offset) = get_author_info(commiter_info)?;

    lines_next(&mut lines)?;
    let message = lines.collect();
    // let tree_hash_be = read_hash_from(stream)?;
    // let number_of_parents = read_u32_from(stream)?;
    // let parents = read_parents_from(number_of_parents, stream)?;
    // let author = Author::read_from(stream)?;
    // let author_timestamp = read_i64_from(stream)?;
    // let author_offset = read_i32_from(stream)?;
    // let committer = Author::read_from(stream)?;
    // let committer_timestamp = read_i64_from(stream)?;
    // let committer_offset = read_i32_from(stream)?;
    // let message = read_string_until(stream, '\n')?;
    Ok((
        tree_hash_be,
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

fn lines_next(lines: &mut std::str::Lines<'_>) -> Result<String, CommandError> {
    let Some(tree_line) = lines.next() else {
        return Err(CommandError::InvalidCommit);
    };
    Ok(tree_line.to_string())
}

fn get_author_info(commiter_info: &str) -> Result<(Author, i64, i32), CommandError> {
    let mut stream = Cursor::new(commiter_info.as_bytes());
    let author = Author::read_from(&mut stream)?;

    let timestamp_str = &read_string_until(&mut stream, ' ')?;
    let timestamp_str = timestamp_str.trim();
    let mut offset_str = String::new();
    stream
        .read_to_string(&mut offset_str)
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;
    let offset_len = offset_str.len();
    let offset_hr = offset_str[..offset_len - 2]
        .parse::<i32>()
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;
    let offset_min = offset_str[offset_len - 2..]
        .parse::<i32>()
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;
    let offset = offset_hr * 60 + offset_min;
    let timestamp = timestamp_str
        .parse::<i64>()
        .map_err(|err| CommandError::FileReadError(err.to_string()))?;

    Ok((author, timestamp, offset))
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

    fn as_commit_mut(&mut self) -> Option<&mut CommitObject> {
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

    fn content(&mut self, db: Option<&mut ObjectsDatabase>) -> Result<Vec<u8>, CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(&mut buf);
        writeln!(stream, "tree {}", self.tree.get_hash_string()?)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        for parent in &self.parents {
            writeln!(stream, "parent {}", parent)
                .map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        }

        write!(stream, "author ").map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        self.author.write_to(&mut stream)?;

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

        write!(stream, "committer ")
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        self.committer.write_to(&mut stream)?;
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
        todo!()
    }

    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
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
}

// impl fmt::Display for CommitObject {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "tree {}\nparent {:?}\nauthor {}\ncommitter {}\n\n{}",
//             self.tree, self.parents, self.author, self.committer, self.message
//         )
//     }
// }

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

/* pub fn write_commit_to_database(
    commit: &mut GitObject,
    tree: &mut Tree,
    logger: &mut Logger,
) -> Result<String, CommandError> {
    write_tree(tree, logger)?;

    let commit_hash = objects_database::write(logger, commit)?;
    Ok(commit_hash)
} */

pub fn write_commit_tree_to_database(
    db: &mut ObjectsDatabase,
    tree: &mut Tree,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let mut boxed_tree: Box<dyn GitObjectTrait> = Box::new(tree.clone());

    db.write(&mut boxed_tree, false, logger)?;
    for (_, child) in tree.get_objects().iter_mut() {
        if let Some(child_tree) = child.as_mut_tree() {
            write_commit_tree_to_database(db, child_tree, logger)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Write};

    use crate::{
        file_compressor::compress, objects::git_object, utils::super_string::SuperStrings,
    };

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

    #[test]
    #[ignore]
    fn write_and_read() {
        let hash_str = "a471637c78c8f67cca05221a942bd7efabb58caa".to_string();
        let hash = hash_str.cast_hex_to_u8_vec().unwrap();
        let mut commit = CommitObject::new(
            vec![],
            "message".to_string(),
            Author::new("name", "email"),
            Author::new("name", "email"),
            1,
            -180,
            Tree::new("".to_string()),
            Some(hash),
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut writer_stream = Cursor::new(&mut buf);
        commit.write_to(&mut writer_stream, None).unwrap();
        let mut reader_stream = Cursor::new(&mut buf);
        // let mut fetched_commit = git_object::read_git_object_from(
        //     &mut reader_stream,
        //     "",
        //     "a471637c78c8f67cca05221a942bd7efabb58caa",
        //     &mut Logger::new_dummy(),
        // )
        // .unwrap();

        // let mut fetched_commit_buf: Vec<u8> = Vec::new();
        // let mut fetched_commit_writer_stream = Cursor::new(&mut fetched_commit_buf);
        // fetched_commit
        //     .write_to(&mut fetched_commit_writer_stream)
        //     .unwrap();

        // assert_eq!(buf, fetched_commit_buf);
    }

    // Write and display
    #[test]
    #[ignore]
    fn write_and_display() {
        let hash = hex_string_to_u8_vec("a471637c78c8f67cca05221a942bd7efabb58caa");
        let mut tree = Tree::new("".to_string());
        tree.set_hash(hash);
        let mut commit = CommitObject::new(
            vec![],
            "message".to_string(),
            Author::new("name", "email"),
            Author::new("name", "email"),
            1,
            -180,
            tree,
            None,
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut writer_stream = Cursor::new(&mut buf);
        commit.write_to(&mut writer_stream, None).unwrap();
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
