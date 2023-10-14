use std::fs::File;
use std::io::Read;

use crate::commands::{stagin_area::StagingArea, command_errors::CommandError};

use super::{author::Author, mode::Mode, tree::Tree};

extern crate chrono;
use chrono::{prelude::*, TimeZone};

#[derive(Clone)]
pub struct Commit{
    hash: String,
    parent: Option<String>, //cambiar en Merge (puede tener varios padres),
    message: String,
    author: Author,
    date: String, //cambiar por Date o TimeStamp
    tree: Tree

}

impl Commit{
    pub fn new(staging_area: StagingArea, message: String, author: Author)-> Result<(), CommandError>{
        let mut parent: Option<String> = None;
        let parent_hash = Commit::get_parent()?;
        if !parent_hash.is_empty(){
            parent = Some(parent_hash)
        }
        let timestamp = Commit::get_timestamp();
        Ok(())
    }

    pub fn change_date(&mut self, date: String){
        self.date = date;
    }

    fn get_timestamp(){
        let timestamp: DateTime<Local> = Local::now();
        // formatteo para que se vea como el de git.
        timestamp.format("%a %b %e %H:%M:%S %Y %z").to_string();
    }

    fn get_parent()-> Result<String, CommandError>{
        let mut branch = String::new();
        let mut parent = String::new();
        let path = ".git/HEAD";
        let Ok(mut head) = File::open(path) else{
            return Err(CommandError::NotGitRepository);
        };

        if head.read_to_string(&mut branch).is_err(){
            return Err(CommandError::FileReadError(path.to_string()));
        }
        let branch = branch.trim();
        let Some(branch) = branch.split(" ").last() else{
            return Err(CommandError::HeadError);
        };
        let branch_path = format!(".git/{}", branch);
        let Ok(mut branch_file) = File::open(path) else{
            return Err(CommandError::FileOpenError(branch_path));
        };
        if branch_file.read_to_string(&mut parent).is_err(){
            return Err(CommandError::FileReadError(branch_path.to_string()));
        }
        let parent = parent.trim();
        Ok(parent.to_string())
    }
}

#[cfg(test)]
mod test{
    use super::*;
    /* #[test]
    fn timestamp(){
        Commit::get_timestamp();
        assert!(false)
    } */
}