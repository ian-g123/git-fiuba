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
    collections::HashMap,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::Path,
};

extern crate chrono;
use chrono::{prelude::*, DateTime};

#[derive(Clone, Debug)]
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
        self.message.clone()
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

        let option_tree = match db {
            Some(db) => {
                logger.log(&format!(
                    "Reading tree hash from database: {}",
                    tree_hash_str
                ));
                let mut tree = db.read_object(&tree_hash_str, logger)?;

                let Some(tree) = tree.as_tree() else {
                    return Err(CommandError::InvalidCommit);
                };
                Some(tree)
            }
            None => None,
        };

        let hash_u8: Option<[u8; 20]> = hash_commit.map(|hash| hex_string_to_u8_vec(&hash));

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

    let mut signature_block = Vec::new();
    let sig_line = lines_next(&mut lines)?;
    if sig_line == "gpgsig -----BEGIN PGP SIGNATURE-----" {
        signature_block.push(sig_line);
        let mut line = lines_next(&mut lines)?;
        while line != " -----END PGP SIGNATURE-----" {
            signature_block.push(line);
            line = lines_next(&mut lines)?;
        }
        signature_block.push(line);

        lines_next(&mut lines)?;
        lines_next(&mut lines)?;
    }
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

    fn content(&mut self, db: Option<&ObjectsDatabase>) -> Result<Vec<u8>, CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = Cursor::new(&mut buf);

        writeln!(stream, "tree {}", u8_vec_to_hex_string(&self.tree_hash))
            .map_err(|err| CommandError::FileWriteError(err.to_string()))?;

        if let Some(db) = db {
            let Some(tree) = self.tree.as_mut() else {
                return Err(CommandError::InvalidCommit);
            };
            let mut tree_box: GitObject = Box::new(tree.clone());
            db.write(&mut tree_box, true, &mut Logger::new_dummy())?;
        };

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

pub fn write_commit_tree_to_database(
    db: &mut ObjectsDatabase,
    tree: &mut Tree,
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let mut boxed_tree: Box<dyn GitObjectTrait> = Box::new(tree.clone());

    db.write(&mut boxed_tree, false, logger)?;
    for (_, (_child_hash, child_obj_opt)) in tree.get_objects().iter_mut() {
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
        let (type_str, len) = git_object::get_type_and_len(&mut reader_stream).unwrap();
        git_object::display_from_stream(
            type_str,
            len,
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
    vec_commits: &mut Vec<(CommitObject, usize, usize)>,
) -> Result<(), CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut writer_stream = Cursor::new(&mut buf);
    let path_to_refs_heads = Path::new("./.git/refs/heads");
    let branchs_commits = get_hashes_for_refs_heads(path_to_refs_heads);
    let mut branchs_commits = match branchs_commits {
        Ok(branchs_commits) => branchs_commits,
        Err(_) => return Err(CommandError::ReadRefsHeadError),
    };
    for commit_with_branch in vec_commits {
        if commit_with_branch.0.is_merge() {
            print_merge_commit_for_log(
                &mut writer_stream,
                &mut commit_with_branch.0,
                &mut branchs_commits,
            )?;
        } else {
            print_normal_commit_for_log(
                &mut writer_stream,
                &mut commit_with_branch.0,
                &mut branchs_commits,
            )?;
        }
    }
    _ = stream.write_all(&buf);
    Ok(())
}

fn get_hashes_for_refs_heads(dir: &Path) -> std::io::Result<HashMap<String, Vec<String>>> {
    let mut hash_branch_hash: HashMap<String, Vec<String>> = HashMap::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut file = File::open(&path)?;
                let mut hash = String::new();
                file.read_to_string(&mut hash)?;
                let branch = path
                    .to_str()
                    .unwrap_or("")
                    .split('/')
                    .collect::<Vec<&str>>()
                    .last()
                    .unwrap_or(&"")
                    .to_string();
                if let Some(vec) = hash_branch_hash.get_mut(&hash) {
                    vec.push(branch);
                } else {
                    hash_branch_hash.insert(hash, vec![branch]);
                }
            }
        }
    }
    Ok(hash_branch_hash)
}

fn print_normal_commit_for_log(
    stream: &mut dyn Write,
    commit: &mut CommitObject,
    branchs_commits: &mut HashMap<String, Vec<String>>,
) -> Result<(), CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut writer_stream = Cursor::new(&mut buf);
    let commit_hash_str = commit.get_hash_string()?;
    _ = write!(writer_stream, "commit {}", commit_hash_str);

    if let Some(branch_vec) = branchs_commits.get(&commit_hash_str) {
        let branch_str = branch_vec.join(", ");
        _ = write!(writer_stream, " {}", branch_str);
    }

    _ = writeln!(writer_stream);
    _ = writeln!(writer_stream, "Author: {}", commit.author);
    _ = writeln!(writer_stream, "Date: {}", commit.get_timestamp_string());
    _ = writeln!(writer_stream, "\n    {}\n", commit.message);
    _ = stream.write_all(&buf);
    Ok(())
}

fn print_merge_commit_for_log(
    stream: &mut dyn Write,
    commit: &mut CommitObject,
    branchs_commits: &mut HashMap<String, Vec<String>>,
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
    let commit_hash_str = commit.get_hash_string()?;
    _ = write!(writer_stream, "commit {}", commit_hash_str);

    if let Some(branch_vec) = branchs_commits.get(&commit_hash_str) {
        let branch_str = branch_vec.join(" ");
        _ = write!(writer_stream, " {}", branch_str);
    }
    _ = writeln!(writer_stream);
    _ = writeln!(writer_stream, "{}", merges);
    _ = writeln!(writer_stream, "Author: {}", commit.author);
    _ = writeln!(writer_stream, "Date: {}", commit.get_timestamp_string());
    _ = writeln!(writer_stream, "\n    {}", commit.message);
    _ = stream.write_all(&buf);
    Ok(())
}

pub fn sort_commits_descending_date(vec_commits: &mut [(CommitObject, String)]) {
    vec_commits.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));
}

pub fn sort_commits_descending_date_and_topo(vec_commits: &mut [(CommitObject, usize, usize)]) {
    vec_commits.sort_by(|a, b| b.2.cmp(&a.2));
    vec_commits.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));
}

pub fn sort_commits_ascending_date_and_topo(vec_commits: &mut [(CommitObject, usize, usize)]) {
    vec_commits.sort_by(|a, b| a.2.cmp(&b.2));
    vec_commits.sort_by(|a, b| a.0.timestamp.cmp(&b.0.timestamp));
}

fn timestamp_to_string(timestamp: i64) -> String {
    let Some(datetime) = DateTime::from_timestamp(timestamp, 0) else {
        return "No tiene".to_string();
    };

    // Formatear la fecha en una cadena de texto
    let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    formatted_date
}
