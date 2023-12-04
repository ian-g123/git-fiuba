use std::{
    fs::{self},
    path::Path,
    process::Command,
};

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, create_test_scene_2,
};

mod common {
    pub mod aux;
}

#[test]
fn test_create_tag() {
    let path = "./tests/data/commands/tag/repo1";
    create_test_scene_2(path);

    // tag -> HEAD

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path.clone()).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("-m")
        .arg("\"message")
        .arg("tag1\"")
        .arg("tag1")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";
    assert_eq!(expected, stdout);

    let tag1_path = format!("{}/.git/refs/tags/tag1", path);

    check_tag_info(
        path,
        &tag1_path,
        &master_commit,
        "commit",
        "tag1",
        "message tag1",
    );

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    // tag -> commit != HEAD

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("-a")
        .arg("tag2")
        .arg("-m")
        .arg("message")
        .arg(master_commit.trim())
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";
    assert_eq!(expected, stdout);

    let tag2_path = format!("{}/.git/refs/tags/tag2", path);

    check_tag_info(
        path,
        &tag2_path,
        &master_commit,
        "commit",
        "tag2",
        "message",
    );

    // Tag exists

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("-a")
        .arg("tag2")
        .arg("-m")
        .arg("message-updated")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "fatal: tag 'tag2' already exists\n";
    assert_eq!(expected, stderr);

    // Tag exists + (-f)

    let tag2_previous_hash = fs::read_to_string(tag2_path.clone()).unwrap();
    let master_commit = fs::read_to_string(master_path).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("-m")
        .arg("message2")
        .arg("-f")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = format!(
        "Updated tag 'tag2' (was {})\n",
        tag2_previous_hash[..6].to_string()
    );
    assert_eq!(expected, stdout);

    check_tag_info(
        path,
        &tag2_path,
        &master_commit,
        "commit",
        "tag2",
        "message2",
    );

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_create_ref() {
    let path = "./tests/data/commands/tag/repo2";
    create_test_scene_2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .arg("no-existe")
        .current_dir(path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = format!("fatal: Failed to resolve 'no-existe' as a valid ref.\n");
    assert_eq!(expected, stderr);

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path.clone()).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";
    assert_eq!(expected, stdout);

    let tag1_path = format!("{}/.git/refs/tags/tag1", path);
    let tag_object = fs::read_to_string(tag1_path).unwrap();

    assert_eq!(master_commit, tag_object);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_delete_tags() {
    let path = "./tests/data/commands/tag/repo3";
    create_test_scene_2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let _result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    let tag1_path = format!("{}/.git/refs/tags/tag1", path);
    let tag1_object = fs::read_to_string(tag1_path).unwrap();

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    let _result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("-a")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let tag2_path = format!("{}/.git/refs/tags/tag2", path);
    let tag2_object = fs::read_to_string(tag2_path).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .arg("no-existe")
        .arg("-d")
        .arg("tag2")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = format!("error: tag 'no-existe' not found.\nDeleted tag tag1 (was {}).\nDeleted tag tag2 (was {}).\n", tag1_object[..7].to_string(), tag2_object[..7].to_string());
    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_list_tags() {
    let path = "./tests/data/commands/tag/repo4";
    create_test_scene_2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("-a")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "tag1\ntag2\n";
    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_tag_branch() {
    let path = "./tests/data/commands/tag/repo5";
    create_test_scene_2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("-b")
        .arg("b1")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("b1")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Tag error: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Tag stdout: {}", stdout);
    let b1_path = format!("{}/.git/refs/heads/b1", path);
    let tag_path = format!("{}/.git/refs/tags/tag2", path);

    let b1_commit = fs::read_to_string(b1_path.clone()).unwrap();

    assert!(Path::new(&tag_path).exists());

    let tag_hash = fs::read_to_string(tag_path).unwrap();
    assert_eq!(tag_hash, b1_commit);
    _ = std::fs::remove_dir_all(format!("{}", path));
}

fn check_tag_info(
    path: &str,
    tag_path: &str,
    object_hash: &str,
    object_type: &str,
    name: &str,
    message: &str,
) {
    assert!(Path::new(&tag_path).exists());

    let tag_hash = fs::read_to_string(tag_path).unwrap();
    println!("tag hash: {}", tag_hash);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("-p")
        .arg(tag_hash.trim())
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stdout:\n{}", stdout);
    let output_lines: Vec<&str> = stdout.split('\n').collect();

    let (_, tag_object_hash) = output_lines[0].split_once(" ").unwrap();
    let (_, tag_object_type) = output_lines[1].split_once(" ").unwrap();
    let (_, tag_name) = output_lines[2].split_once(" ").unwrap();
    let tag_message = output_lines[5];

    assert_eq!(object_hash, tag_object_hash);
    assert_eq!(object_type, tag_object_type);
    assert_eq!(name, tag_name);
    assert_eq!(message, tag_message);
}
