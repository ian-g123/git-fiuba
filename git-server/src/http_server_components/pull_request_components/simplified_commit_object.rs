use git_lib::objects::commit_object::CommitObject;
use serde::{Deserialize, Serialize};

use super::simplified_author::SimplfiedAuthor;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplifiedCommitObject {
    author: SimplfiedAuthor,
    commiter: SimplfiedAuthor,
    message: String,
    tree: String,
    parents: Vec<String>,
}
impl SimplifiedCommitObject {
    pub fn from_commit(commit: CommitObject) -> Self {
        SimplifiedCommitObject {
            author: SimplfiedAuthor::from_author(commit.get_author(), commit.get_author_date()),
            commiter: SimplfiedAuthor::from_author(
                commit.get_committer(),
                commit.get_committer_date(),
            ),
            message: commit.get_message(),
            tree: commit.get_tree_hash_string(),
            parents: commit.get_parents(),
        }
    }
}
