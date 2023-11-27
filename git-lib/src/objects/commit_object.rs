use super::{
    author::Author,
    git_object::{GitObject, GitObjectTrait},
    tree::Tree,
};
use crate::{
    command_errors::CommandError,
    logger::Logger,
    objects_database::ObjectsDatabase,
    utils::{
        aux::{get_sha1, hex_string_to_u8_vec, read_string_until},
        super_string::u8_vec_to_hex_string,
    },
};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
};

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
    tree: Option<Tree>,
    tree_hash: [u8; 20],
    hash: Option<[u8; 20]>,
}

impl CommitObject {
    /// Crea un objeto Commit.
    pub fn new_from_tree(
        parent: Vec<String>,
        message: String,
        author: Author,
        committer: Author,
        timestamp: i64,
        offset: i32,
        mut tree: Tree,
        hash: Option<[u8; 20]>,
    ) -> Result<Self, CommandError> {
        let tree_hash = tree.get_hash()?;
        Ok(Self {
            parents: parent,
            message,
            author,
            committer,
            timestamp,
            offset,
            hash,
            tree: Some(tree),
            tree_hash,
        })
    }

    pub fn get_parents(&self) -> Vec<String> {
        self.parents.clone()
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn get_timestamp_string(&self) -> String {
        timestamp_to_string(self.timestamp)
    }

    /// Devuelve el hash del tree del Commit.
    pub fn get_tree_hash_string(&mut self) -> Result<String, CommandError> {
        Ok(u8_vec_to_hex_string(&self.tree_hash))
    }

    pub fn get_message(&self) -> String {
        return self.message.clone();
    }

    pub fn get_author(&self) -> Author {
        self.author.clone()
    }

    /// Crea un Commit a partir de la infromación leída del stream.
    pub fn read_from(
        db: Option<&ObjectsDatabase>,
        stream: &mut dyn Read,
        logger: &mut Logger,
        hash_commit: Option<String>,
    ) -> Result<GitObject, CommandError> {
        let (tree_hash, parents, author, author_timestamp, author_offset, committer, _, _, message) =
            read_commit_info_from(stream)?;

        let tree_hash_str = u8_vec_to_hex_string(&tree_hash);

        logger.log(&format!(
            "Reading tree hash from database: {}",
            tree_hash_str
        ));

        let option_tree = match db {
            Some(db) => {
                let mut tree = db.read_object(&tree_hash_str, logger)?;
                logger.log(&format!(
                    "tree content en read_from : {}",
                    String::from_utf8_lossy(&(tree.to_owned().content(None)?))
                ));

                let Some(tree) = tree.as_tree() else {
                    return Err(CommandError::InvalidCommit);
                };
                Some(tree)
            }
            None => None,
        };

        let hash_u8: Option<[u8; 20]> = match hash_commit {
            Some(hash) => Some(hex_string_to_u8_vec(&hash)),
            None => None,
        };

        Ok(Box::new(CommitObject {
            tree: option_tree,
            tree_hash,
            parents,
            author,
            committer,
            message,
            timestamp: author_timestamp,
            offset: author_offset,
            hash: hash_u8,
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

    pub fn get_tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    pub fn get_tree_some_or_err(&self) -> Result<Tree, CommandError> {
        match self.tree {
            Some(ref tree) => Ok(tree.clone()),
            None => Err(CommandError::InvalidCommit),
        }
    }

    fn is_merge(&self) -> bool {
        self.parents.len() > 1
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

pub fn lines_next(lines: &mut std::str::Lines<'_>) -> Result<String, CommandError> {
    let Some(line) = lines.next() else {
        return Err(CommandError::InvalidCommit);
    };
    Ok(line.to_string())
}

pub fn get_author_info(commiter_info: &str) -> Result<(Author, i64, i32), CommandError> {
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
pub fn offset_str(minutes: i32) -> Result<String, CommandError> {
    let sign = if minutes < 0 { "-" } else { "+" };
    let hours = minutes.abs() / 60;
    let minutes = minutes.abs() % 60;
    Ok(format!("{}{:02}{:02}", sign, hours, minutes))
}

/// Devuelve un vector con el hash de cada commit padre.
fn _read_parents_from(
    number_of_parents: u32,
    stream: &mut dyn Read,
) -> Result<Vec<String>, CommandError> {
    let mut parents = Vec::<String>::new();
    for _ in 0..number_of_parents {
        let parent_hash_be = _read_hash_from(stream)?;
        let parent_hash = u8_vec_to_hex_string(&parent_hash_be);
        parents.push(parent_hash);
    }
    Ok(parents)
}

/// Lee el hash del stream y lo devuelve en formato  Vec<u8>
fn _read_hash_from(stream: &mut dyn Read) -> Result<[u8; 20], CommandError> {
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

    fn as_mut_commit(&mut self) -> Option<&mut CommitObject> {
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

        let Some(tree) = self.tree.as_mut() else {
            return Err(CommandError::InvalidCommit);
        };

        writeln!(stream, "tree {}", tree.get_hash_string()?)
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        match db {
            Some(db) => {
                let mut tree_box: GitObject = Box::new(tree.clone());
                db.write(&mut tree_box, true, &mut Logger::new_dummy())?;
            }
            None => {}
        };

        // if self.tree.is_some() {
        //     writeln!(
        //         stream,
        //         "tree {}",
        //         self.tree.as_mut().unwrap().get_hash_string()?
        //     )
        //     .map_err(|err| CommandError::FileWriteError(err.to_string()))?;
        // }

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
fn _get_date(line: &mut Vec<&str>) -> Result<DateTime<Local>, CommandError> {
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
    for (_, (child_hash, child_obj_opt)) in tree.get_objects().iter_mut() {
        let Some(child) = child_obj_opt else {
            return Err(CommandError::ShallowTree);
        };
        if let Some(child_tree) = child.as_mut_tree() {
            write_commit_tree_to_database(db, child_tree, logger)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {

    use crate::{objects::git_object, utils::super_string::SuperStrings};

    use super::*;

    #[test]
    #[ignore]
    fn write_and_read() {
        let hash_str = "a471637c78c8f67cca05221a942bd7efabb58caa".to_string();
        let hash = hash_str.cast_hex_to_u8_vec().unwrap();
        let mut commit = CommitObject::new_from_tree(
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
        let _reader_stream = Cursor::new(&mut buf);
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
        let mut commit = CommitObject::new_from_tree(
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

pub fn print_for_log(
    stream: &mut dyn Write,
    vec_commits: &mut Vec<(CommitObject, Option<String>)>,
) -> Result<(), CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut writer_stream = Cursor::new(&mut buf);
    for commit_with_branch in vec_commits {
        if commit_with_branch.0.is_merge() {
            print_merge_commit_for_log(&mut writer_stream, &mut commit_with_branch.0)?;
        } else {
            print_normal_commit_for_log(&mut writer_stream, &mut commit_with_branch.0)?;
        }
    }
    _ = stream.write_all(&buf);
    Ok(())
}

fn print_normal_commit_for_log(
    stream: &mut dyn Write,
    commit: &mut CommitObject,
) -> Result<(), CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut writer_stream = Cursor::new(&mut buf);
    _ = writeln!(writer_stream, "commit {}", commit.get_hash_string()?);
    _ = writeln!(writer_stream, "Author: {}", commit.author);
    _ = writeln!(writer_stream, "Date: {}", commit.timestamp);
    _ = writeln!(writer_stream, "\n\t{}", commit.message);
    _ = stream.write_all(&buf);
    Ok(())
}

fn print_merge_commit_for_log(
    stream: &mut dyn Write,
    commit: &mut CommitObject,
) -> Result<(), CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut writer_stream = Cursor::new(&mut buf);
    let mut merges = "Merge:".to_string();
    for parent in &commit.parents {
        if parent.len() > 7 {
            merges.push_str(&format!(" {}", &parent[..7]));
        } else {
            return Err(CommandError::InvalidCommit);
        }
    }
    _ = writeln!(writer_stream, "commit {}", commit.get_hash_string()?);
    _ = writeln!(writer_stream, "{}", merges);
    _ = writeln!(writer_stream, "Author: {}", commit.author);
    _ = writeln!(writer_stream, "Date: {}", commit.timestamp);
    _ = writeln!(writer_stream, "\n\t{}", commit.message);
    _ = stream.write_all(&buf);
    Ok(())
}

// pub fn read_from_for_log(
//     db: &ObjectsDatabase,
//     stream: &mut dyn Read,
//     logger: &mut Logger,
//     hash_commit: &String,
// ) -> Result<CommitObject, CommandError> {
//     //let mut tree = db.read_object(&hash_commit)?;
//     get_type_and_len(stream)?;

//     let (_, parents, author, author_timestamp, author_offset, committer, _, _, message) =
//         read_commit_info_from(stream)?;

//     Ok(CommitObject {
//         tree: None,
//         parents,
//         author,
//         committer,
//         message,
//         timestamp: author_timestamp,
//         offset: author_offset,
//         hash: Some(hex_string_to_u8_vec(hash_commit)),
//     })
// }

pub fn sort_commits_descending_date2(
    vec_commits: &mut Vec<(CommitObject, Option<String>)>,
    parents_hash: &mut HashMap<String, HashSet<String>>,
) {
    vec_commits.sort_by(|a, b| {
        // Comparar por timestamp en orden descendente
        let timestamp_order = b.0.timestamp.cmp(&a.0.timestamp);
        if timestamp_order != Ordering::Equal {
            return timestamp_order;
        }

        // Si los timestamps son iguales, comparar por inclusión en el HashSet
        match (&a.1, &b.1) {
            (Some(parent_a), Some(parent_b)) => {
                println!("parent_a: {:?}, parent_b: {:?}", parent_a, parent_b);
                if parents_hash[parent_a].contains(parent_b) {
                    Ordering::Greater
                } else if parents_hash[parent_b].contains(parent_a) {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });
}

pub fn sort_commits_descending_date(vec_commits: &mut Vec<(CommitObject, Option<String>)>) {
    vec_commits.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));
}

pub fn sort_commits_ascending_date(vec_commits: &mut Vec<(CommitObject, Option<String>)>) {
    vec_commits.sort_by(|a, b| a.0.timestamp.cmp(&b.0.timestamp));
}

fn timestamp_to_string(timestamp: i64) -> String {
    // let duration = Duration::from_secs(timestamp as u64);
    let Some(datetime) = DateTime::from_timestamp(timestamp, 0) else {
        return "No tiene".to_string();
    };

    // Formatear la fecha en una cadena de texto
    let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    formatted_date
}
