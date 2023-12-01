use std::io::Cursor;

use git_lib::{git_repository::GitRepository, logger::Logger, objects::git_object::GitObjectTrait};

#[test]
#[ignore = "git-lib/tests/data/packfile-database.zip must be extracted before running the test"]
fn test() {
    let repo_path = "tests/data/packfile-database";
    let output = Vec::new();

    let mut output_writer = Cursor::new(output);
    let mut repo = GitRepository::open(repo_path, &mut output_writer).unwrap();
    let commits = repo.get_log(true).unwrap();
    let (mut commit, _) = commits.get(0).unwrap().to_owned();
    assert_eq!(commit.get_message(), "initialcommit");
    let tree = repo
        .db()
        .unwrap()
        .to_owned()
        .read_object(
            &commit.get_tree_hash_string().unwrap(),
            &mut Logger::new_dummy(),
        )
        .unwrap()
        .as_mut_tree()
        .unwrap()
        .to_owned();
    let tree_entries = tree.get_objects();
    assert_eq!(tree_entries.len(), 1);
    println!("{:?}", tree_entries);
    let (hash, object) = tree_entries.get("file").unwrap();
    let mut object = object.to_owned().unwrap().as_mut_blob().unwrap().to_owned();
    assert_eq!(
        object.get_hash_string().unwrap(),
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
    );
    assert_eq!(&object.get_hash().unwrap(), hash);
    assert_eq!(object.content(None).unwrap(), "".as_bytes());
    let _ = std::fs::remove_dir_all(repo_path);
}
