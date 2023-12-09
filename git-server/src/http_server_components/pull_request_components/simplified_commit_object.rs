use git_lib::objects::commit_object::CommitObject;
use serde::{Deserialize, Serialize};

use super::simplified_author::SimplfiedAuthor;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplifiedCommitObject {
    author: SimplfiedAuthor,
    committer: SimplfiedAuthor,
    message: String,
    tree: String,
    parents: Vec<String>,
}
impl SimplifiedCommitObject {
    pub fn from_commit(commit: CommitObject) -> Self {
        SimplifiedCommitObject {
            author: SimplfiedAuthor::from_author(commit.get_author(), commit.get_author_date()),
            committer: SimplfiedAuthor::from_author(
                commit.get_committer(),
                commit.get_committer_date(),
            ),
            message: commit.get_message(),
            tree: commit.get_tree_hash_string(),
            parents: commit.get_parents(),
        }
    }

    pub fn to_string_plain_format(&self) -> String {
        let mut string = String::new();
        string += &format!("tree {}\n", self.tree);
        for parent_hash in self.parents.iter() {
            string += &format!("parent {}\n", parent_hash);
        }
        string += &format!("author {}\n", self.author.to_string());
        string += &format!("committer {}\n", self.committer.to_string());
        string += &format!("\n{}\n", self.message);
        string
    }
}
