use git_lib::{
    command_errors::CommandError,
    objects::{commit_object::CommitObject, git_object::GitObjectTrait},
};
use serde::{Deserialize, Serialize};

use super::simplified_author::SimplfiedAuthor;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplifiedCommitObject {
    hash: String,
    author: SimplfiedAuthor,
    committer: SimplfiedAuthor,
    message: String,
    tree: String,
    parents: Vec<String>,
}
impl SimplifiedCommitObject {
    /// Recibe un CommitObject y crea un SimplifiedCommitObject a partir de éste.
    pub fn from_commit(mut commit: CommitObject) -> Result<Self, CommandError> {
        Ok(SimplifiedCommitObject {
            hash: commit.get_hash_string()?,
            author: SimplfiedAuthor::from_author(commit.get_author(), commit.get_author_date()),
            committer: SimplfiedAuthor::from_author(
                commit.get_committer(),
                commit.get_committer_date(),
            ),
            message: commit.get_message(),
            tree: commit.get_tree_hash_string(),
            parents: commit.get_parents(),
        })
    }

    /// Convierte el SimplifiedCommitObject a texto plano.
    pub fn to_string_plain_format(&self) -> String {
        let mut string = String::new();
        string += &format!("{}\n", self.hash);
        string += &format!("tree {}\n", self.tree);
        for parent_hash in self.parents.iter() {
            string += &format!("parent {}\n", parent_hash);
        }
        string += &format!("author {}\n", self.author);
        string += &format!("committer {}\n", self.committer);
        string += &format!("\n{}\n", self.message);
        string
    }
}
