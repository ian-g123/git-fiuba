use std::collections::HashMap;
use std::env::current_dir;
use std::fmt;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::aux::{
    get_sha1, hex_string_to_u8_vec, read_string_from, u8_vec_to_hex_string, SuperStrings,
};
use super::blob::Blob;
use super::git_object::{GitObject, GitObjectTrait};
use super::{author::Author, tree::Tree};
use crate::commands::command_errors::CommandError;
use crate::commands::file_compressor::extract;

extern crate chrono;
use chrono::{prelude::*, DateTime, LocalResult, Offset, TimeZone};

#[derive(Clone)]
pub struct CommitObject {
    //hash: String,
    parents: Vec<String>,
    message: String,
    author: Author,
    committer: Author,
    date: DateTime<Local>,
    tree: String,
}

impl CommitObject {
    /// Crea un Commit a partir de los cambios del Staging Area.
    // pub fn new(index: StagingArea, message: String, author: Author) -> Result<(), CommandError> {
    //     let mut parent: Option<String> = None;
    //     let parent_hash = Commit::get_parent()?;

    //     if !parent_hash.is_empty() {
    //         parent = Some(parent_hash)
    //     }

    //     let timestamp = get_timestamp();

    //     let tree = CommitTree::new(index.files, parent)?;

    //     // falta hash y author

    //     Ok(())
    // }

    pub fn new(
        parent: Vec<String>, //cambiar en Merge (puede tener varios padres),
        message: String,
        author: Author,
        committer: Author,
        date: DateTime<Local>,
        tree: String,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            parents: parent,
            message,
            author,
            committer,
            date,
            tree,
        })
    }

    /// Cambia la fecha y hora del Commit.
    // pub fn change_date(&mut self, date: String) {
    //     self.date = date;
    // }

    /// Obtiene el hash del Commit padre. Si no tiene p
    fn get_parent() -> Result<String, CommandError> {
        let mut parent = String::new();
        let branch = get_current_branch()?;
        let branch_path = format!(".git/{}", branch);
        let Ok(mut branch_file) = File::open(branch_path.clone()) else {
            return Ok(parent);
        };

        if branch_file.read_to_string(&mut parent).is_err() {
            return Err(CommandError::FileReadError(branch_path.to_string()));
        }

        let parent = parent.trim();
        Ok(parent.to_string())
    }

    // pub fn to_string(&self) -> String {
    //     let mut output = String::new();
    //     output.push_str(&format!("tree {}\n", "tree_hash"));
    //     // output.push_str(&format!("tree {}\n", self.tree.get_hash()));
    //     if let Some(parent) = &self.parent {
    //         output.push_str(&format!("parent {}\n", parent));
    //     }
    //     output.push_str(&format!(
    //         "author {} {} {}\n",
    //         self.author.to_string(),
    //         self.date.timestamp(),
    //         self.date.offset()
    //     ));
    //     output.push_str(&format!(
    //         "committer {} {} {}\n",
    //         self.committer.to_string(),
    //         self.date.timestamp(),
    //         self.date.offset()
    //     ));
    //     output.push_str(&format!("\n{}\n", self.message));
    //     output
    // }

    // pub fn from_string(string: String) -> Result<Self, CommandError> {
    //     let mut lines = string.lines();
    //     let mut line = lines.next().unwrap().split_whitespace();
    //     if line.next().unwrap() != "tree" {
    //         return Err(CommandError::InvalidCommit);
    //     }
    //     let tree_hash = line.next().unwrap();
    //     let mut line = lines.next().unwrap().split_whitespace();
    //     let mut parents = Vec::<String>::new();
    //     while line.next().unwrap() == "parent" {
    //         let parent = line.next().unwrap().to_string();
    //         parents.push(parent);
    //         line = lines.next().unwrap().split_whitespace();
    //     }
    //     let mut line: Vec<&str> = line.collect();
    //     if line.remove(0) != "author" {
    //         return Err(CommandError::InvalidCommit);
    //     }

    //     let date = get_date(&mut line)?;

    //     let author = Author::from_strings(&mut line)?;
    //     let mut line: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    //     if line.remove(0) != "committer" {
    //         return Err(CommandError::InvalidCommit);
    //     }

    //     let date = get_date(&mut line)?;

    //     let committer = Author::from_strings(&mut line)?;
    //     //skip line
    //     lines.next();
    //     let message = lines.collect::<Vec<&str>>().join("\n");
    //     let hash = tree.get_hash_interno();
    //     Ok(CommitObject {
    //         parents,
    //         author,
    //         committer,
    //         message,
    //         date,
    //         tree: hash,
    //     })
    // }

    fn read_from(reader_stream: &mut dyn Read) -> Result<GitObject, CommandError> {
        let mut tree_hash_be = [0; 20];
        reader_stream
            .read_exact(&mut tree_hash_be)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let tree_hash = u8_vec_to_hex_string(&tree_hash_be);
        let mut parents_len_be = [0; 4];
        reader_stream
            .read_exact(&mut parents_len_be)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let parents_len = u32::from_be_bytes(parents_len_be);
        let mut parents = Vec::<String>::new();
        for _ in 0..parents_len {
            let mut parent_hash_be = [0; 20];
            reader_stream
                .read_exact(&mut parent_hash_be)
                .map_err(|error| CommandError::FileReadError(error.to_string()))?;
            let parent_hash = u8_vec_to_hex_string(&parent_hash_be);
            parents.push(parent_hash);
        }
        let author = Author::read_from(reader_stream)?;
        let _date = read_datetime_from(reader_stream)?;

        let committer = Author::read_from(reader_stream)?;
        let date = read_datetime_from(reader_stream)?;

        let message = read_string_from(reader_stream)?;
        Ok(Box::new(Self {
            tree: tree_hash,
            parents,
            author,
            committer,
            message,
            date,
        }))
    }
}

fn read_datetime_from(reader_stream: &mut dyn Read) -> Result<DateTime<Local>, CommandError> {
    let mut timestamp_be = [0; 8];
    reader_stream
        .read_exact(&mut timestamp_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let timestamp = i64::from_be_bytes(timestamp_be);

    let mut offset_be = [0; 4];
    reader_stream
        .read_exact(&mut offset_be)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;
    let offset = i32::from_be_bytes(offset_be);
    let Some(offset) = chrono::FixedOffset::east_opt(offset * 60) else {
        return Err(CommandError::InvalidCommit);
    };
    let Some(datetime_with_offset) = DateTime::<Utc>::from_timestamp(1431648000, 0) else {
        return Err(CommandError::InvalidCommit);
    };

    let date: DateTime<Local> = datetime_with_offset.with_timezone(&TimeZone::from_offset(&offset));

    Ok(date)
}

/*
    // Parsear el offset a un objeto FixedOffset
    let offset = FixedOffset::west(3 * 3600);  // 3 horas hacia el oeste

    // Crear un DateTime<FixedOffset> a partir del timestamp y el offset
    let datetime_with_offset = offset.timestamp(timestamp, 0);

    // Convertir el DateTime<FixedOffset> a DateTime<Local>
    let local_datetime = datetime_with_offset.with_timezone(&Local);
*/

impl GitObjectTrait for CommitObject {
    fn type_str(&self) -> String {
        "commit".to_string()
    }

    fn mode(&self) -> super::mode::Mode {
        todo!()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&hex_string_to_u8_vec(&self.tree));
        let parents_len_be = self.parents.len().to_be_bytes();
        buf.extend_from_slice(&parents_len_be);
        for parent in &self.parents {
            buf.extend_from_slice(&hex_string_to_u8_vec(parent));
        }
        let mut stream = Cursor::new(&mut buf);
        stream.seek(SeekFrom::End(0));
        self.author.write_to(&mut stream)?;

        let timestamp = self.date.timestamp();
        let offset = self.date.offset().local_minus_utc() / 60;
        write_timestamp(&mut stream, timestamp, offset)?;

        self.committer.write_to(&mut stream)?;
        write_timestamp(&mut stream, timestamp, offset)?;

        self.message.write_to(&mut stream)?;
        Ok(buf)
    }

    /*
    tree 03aa1c401b80e7db6e4d698a9a1c97563b14777c
    parent f245dc05ccca1252f9538ebdcf8ea678f1beed0a
    parent 0f8d48b7a6c59cdee529b66b713a017c7d03219d
    author Patricio Tourne Passarino <111926376+ptourne@users.noreply.github.com> 1697312105 -0300
    committer GitHub <noreply@github.com> 1697312105 -0300
    gpgsig -----BEGIN PGP SIGNATURE-----

     wsBcBAABCAAQBQJlKu1pCRBK7hj4Ov3rIwAAFX4IAJ8861LAbS5tdrK6JiqiZqZZ
     N41xrhcxEGi3NoKyglnTu+/8KPTKLEuJyTzbZ8AJW/+ofVhr2R+8iRk9N1AFg4EV
     0KyMmjoDpUHv+AsEfMqmvI/gSbyccG9u+oUlgm9D+UgxB7JrfoBLRnQN/zILyb+i
     NpzD4FNS5uis6hq3aKTisfibHrLqLFDBjka+rokkiYZeSGAbZyfw+hKrdsfR/ztn
     epi3S9sgl4xM1lW1fvLtP+rrPlA0m0cm12CkWrZiejNmA47iuxPyhXuPJoUOwVlY
     PE2ANcOojB8Y6FAsKa8EXhQ1zz9NoqKetcRrUIum/6WJeq5Sgb5HaK4pq/AAWT4=
     =DtV/
     -----END PGP SIGNATURE-----


    Merge pull request #8 from taller-1-fiuba-rust/init
         */

    fn to_string_priv(&self) -> String {
        todo!()
    }

    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        None
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn get_hash(&self) -> Result<[u8; 20], CommandError> {
        todo!()
    }
}

fn write_timestamp(
    stream: &mut dyn Write,
    timestamp: i64,
    offset: i32,
) -> Result<(), CommandError> {
    stream
        .write_all(&timestamp.to_be_bytes())
        .map_err(|_| CommandError::InvalidCommit)?;
    stream
        .write_all(&offset.to_be_bytes())
        .map_err(|_| CommandError::InvalidCommit)?;
    Ok(())
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

/// Obtiene la rama actual. Si no se puede leer de ".git/HEAD", se devuelve error.
fn get_current_branch() -> Result<String, CommandError> {
    let mut branch = String::new();
    let mut parent = String::new();
    let path = ".git/HEAD";
    let Ok(mut head) = File::open(path) else {
        return Err(CommandError::NotGitRepository);
    };

    if head.read_to_string(&mut branch).is_err() {
        return Err(CommandError::FileReadError(path.to_string()));
    }

    let branch = branch.trim();
    let Some(branch) = branch.split(" ").last() else {
        return Err(CommandError::HeadError);
    };
    Ok(branch.to_string())
}

/// Obtiene el directorio actual.
fn get_current_dir() -> Result<PathBuf, CommandError> {
    let Ok(current_dir) = current_dir() else {
        return Err(CommandError::NotGitRepository);
    };
    Ok(current_dir)
}

struct CommitTree {
    objects: HashMap<String, GitObject>,
}

impl CommitTree {
    /// Crea un Tree que contiene los cambios del Staging Area, así como los archivos del working tree
    /// que se encuentran en el commit anterior. Devuelve error si la operación falla.
    fn new(index: HashMap<String, String>, parent: Option<String>) -> Result<Tree, CommandError> {
        let path = get_current_dir()?;
        let path_name = get_path_name(path)?;
        let mut objects = HashMap::<String, GitObject>::new();
        // Self::compare(path_name.clone(), &index, &mut objects, &parent)?;
        let mut tree = Self::create_tree(&path_name, objects)?;
        Self::add_new_files(&index, &mut tree)?;
        Ok(tree)
    }

    /// Agrega los archivos nuevos que están en el Staging Area, pero no en el Working Tree.
    fn add_new_files(index: &HashMap<String, String>, tree: &mut Tree) -> Result<(), CommandError> {
        for (path, hash) in index {
            tree.add_blob(path, hash)?;
        }
        Ok(())
    }

    /// Compara las carpetas y archivos del Working Tree y el Staging Area. (falta refactor)
    // fn compare(
    //     path_name: String,
    //     index: &HashMap<String, String>,
    //     objects: &mut HashMap<String, GitObject>,
    //     parent: &Option<String>,
    // ) -> Result<(), CommandError> {
    //     let path = Path::new(&path_name);

    //     let Ok(entries) = fs::read_dir(path.clone()) else {
    //         return Err(CommandError::InvalidDirectory);
    //     };
    //     for entry in entries {
    //         let Ok(entry) = entry else {
    //             return Err(CommandError::InvalidDirectoryEntry);
    //         };
    //         let entry_path = entry.path();
    //         let entry_name = get_path_name(entry_path.clone())?;

    //         if entry_path.is_dir() {
    //             let mut objects = HashMap::<String, GitObject>::new();
    //             Self::compare(entry_name.clone(), index, &mut objects, parent)?;
    //             if !index.is_empty() {
    //                 let tree = Self::create_tree(&entry_name, objects.to_owned())?;
    //                 _ = objects.insert(entry_name, Box::new(tree));
    //                 return Ok(());
    //             }
    //         } else {
    //             let result = Self::compare_entry(&path_name, index, parent)?;
    //             if let Some(blob) = result {
    //                 _ = objects.insert(blob.get_hash(), Box::new(blob));
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    /// Crea un Tree.
    fn create_tree(
        path: &String,
        objects: HashMap<String, GitObject>,
    ) -> Result<Tree, CommandError> {
        Ok(Tree::new(path.to_owned()))
    }

    /// Compara un archivo del WorkingTree con el Índex. Si el archivo está en la Staging Area,
    /// se guardan las modificaciones presentes en la misma al Tree. Si el archivo no está en
    /// la Staging Area, pero fue registrado en el commit anterior, se agrega el archivo sin
    /// modificaciones al Tree.
    // fn compare_entry(
    //     path: &String,
    //     index: &HashMap<String, String>,
    //     parent: &Option<String>,
    // ) -> Result<Option<Blob>, CommandError> {
    //     let mut blob: Blob;
    //     if index.contains_key(path) {
    //         let Some(hash) = index.get(path) else {
    //             return Err(CommandError::FileNotFound(path.to_string()));
    //         };
    //         blob = Blob::new_from_hash(hash.to_string(), path.to_owned())?;
    //         return Ok(Some(blob));
    //     }
    //     let hash = get_sha1(path.to_owned(), "blob".to_string(), false)?;

    //     if let Some(parent) = parent {
    //         let found = Self::search_parent_commit(parent.to_string(), hash)?;
    //         if found {
    //             blob = Blob::new_from_path(path.to_owned())?;
    //             return Ok(Some(blob));
    //         }
    //     }

    //     Ok(None)
    // }

    /// Busca el contenido de un archivo en la Base de Datos y lo devuelve. Si no existe, devuelve error.
    fn read_content(hash: String) -> Result<Vec<u8>, CommandError> {
        let mut data: Vec<u8> = Vec::new();
        let path = format!(
            ".git/objects/{}/{}",
            hash[..2].to_string(),
            hash[2..].to_string()
        );
        let Ok(mut tree_file) = File::open(&path) else {
            return Err(CommandError::FileNotFound(path));
        };
        if tree_file.read_to_end(&mut data).is_err() {
            return Err(CommandError::FileReadError(path));
        }
        Ok(data)
    }

    /// Busca en el Commit padre el 'blob_hash'. Si lo encuentra, devuelve true. Si el contenido
    /// del 'parent_hash' no se puede leer o descomprimir, devuelve error.
    fn search_parent_commit(parent_hash: String, blob_hash: String) -> Result<bool, CommandError> {
        let path = format!(
            ".git/objects/{}/{}",
            parent_hash[..2].to_string(),
            parent_hash[2..].to_string()
        );
        let data = Self::read_content(parent_hash)?;
        let data = extract(&data)?;
        let buf = String::from_utf8_lossy(&data).to_string();
        let lines: Vec<&str> = buf.split_terminator("\n").collect();
        for line in lines {
            let info: Vec<&str> = line.split_terminator(" ").collect();
            let (obj_type, obj_hash) = (info[1], info[2]);
            if obj_hash == blob_hash {
                return Ok(true);
            }
            if obj_type == "tree" {
                return Self::search_parent_commit(obj_hash.to_string(), blob_hash);
            }
        }
        Ok(false)
    }
}

/// Devuelve el nombre de un archivo o directorio dado un PathBuf.
fn get_path_name(path: PathBuf) -> Result<String, CommandError> {
    let Some(path_name) = path.to_str() else {
        return Err(CommandError::InvalidDirectoryEntry);
    };
    Ok(path_name.to_string())
}

/// Obtiene la fecha y hora actuales.
fn get_timestamp() {
    let timestamp: DateTime<Local> = Local::now();
    // formateo para que se vea como el de git.
    timestamp.format("%a %b %e %H:%M:%S %Y %z").to_string();
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use crate::commands::file_compressor::compress;

    use super::*;
    /* #[test]
    fn timestamp(){
        Commit::get_timestamp();
        assert!(false)
    } */

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
    fn search_parent_commit_test() {
        if write().is_err() {
            assert!(false, "Falló el write");
        }
        assert!(matches!(
            CommitTree::search_parent_commit(
                "e3540872766f87b1de467a5e867d656a6e6fe959".to_string(),
                "5da01b81e6f2c1926d9e6df32dc160dfe5326239".to_string()
            ),
            Ok(true)
        ));
    }

    // Write unit tests for write to and read from for commits:
    #[test]
    fn write_and_read() {
        // datetime for 1970-01-01 00:00:00 UTC
        let date = Local::now();
        let commit = CommitObject::new(
            vec![],
            "message".to_string(),
            Author::new("name", "email"),
            Author::new("name", "email"),
            date.into(),
            "a471637c78c8f67cca05221a942bd7efabb58c".to_string(),
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut writer_stream = Cursor::new(&mut buf);
        commit.write_to(&mut writer_stream);
        let mut reader_stream = Cursor::new(&mut buf);
        let fetched_commit = CommitObject::read_from(&mut reader_stream);

        let mut fetched_commit_buf: Vec<u8> = Vec::new();
        let mut fetched_commit_writer_stream = Cursor::new(&mut fetched_commit_buf);
        commit.write_to(&mut fetched_commit_writer_stream);

        assert_eq!(buf, fetched_commit_buf);
    }
}
