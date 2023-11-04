use common::aux::create_base_scene;
use git::commands::branch;

mod common {
    pub mod aux;
}

use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    process::Command,
};

use crate::common::aux::{change_testfile_content, create_test_scene_1};

#[test]
fn test_create_branch() {
    let path = "./tests/data/commands/branch/repo1";

    create_test_scene_1(path);

    // crear rama cuando no hay ningún commit

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "fatal: Not a valid object name: 'master'.\n".to_string();
    assert_eq!(expected, stderr);

    // crear rama a partir de HEAD

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
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
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch1_path = format!("{}/.git/refs/heads/branch1", path);
    println!("Branch1: {}", branch1_path);
    let master_path = format!("{}/.git/refs/heads/master", path);

    assert!(Path::new(&branch1_path).exists());

    let master_commit = fs::read_to_string(master_path).unwrap();
    let branch1_commit = fs::read_to_string(branch1_path).unwrap();

    assert_eq!(master_commit, branch1_commit);

    // crear rama existente

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "fatal: A branch named 'branch1' already exists.\n".to_string();

    assert_eq!(stderr, expected);

    change_testfile_content(path);

    // crear rama a partir de otra

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git") // nuevo commit en master
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch2")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch2_path = format!("{}/.git/refs/heads/branch2", path);
    println!("Branch2: {}", branch2_path);

    assert!(Path::new(&branch2_path).exists());

    let branch2_commit = fs::read_to_string(branch2_path).unwrap();

    assert_eq!(branch2_commit, branch1_commit);

    // crear rama a partir de un commit

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch3")
        .arg(master_commit.clone()) // es el primer commit de master
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch3_path = format!("{}/.git/refs/heads/branch3", path);
    println!("Branch3: {}", branch3_path);

    assert!(Path::new(&branch3_path).exists());

    let branch3_commit = fs::read_to_string(branch3_path).unwrap();

    assert_eq!(branch3_commit, master_commit);

    // crear rama a partir de un objeto inexistente

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch3")
        .arg("inexistente") // es el primer commit de master
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let expected = "fatal: Not a valid object name: 'inexistente'.\n".to_string();

    assert_eq!(stderr, expected);

    //_ = fs::remove_dir_all(format!("{}", path));
}
