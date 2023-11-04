use common::aux::create_base_scene;
use git::commands::branch;

mod common {
    pub mod aux;
}

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::Command,
};

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, change_testfile_content,
    create_test_scene_1, create_test_scene_2,
};

#[test]
fn test_update_pathspec() {
    let path = "./tests/data/commands/checkout/repo1";

    create_test_scene_2(path);
    // update file untracked
    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = "error: pathspec 'dir/testfile1.txt' did not match any file(s) known to git\n";
    let stderr = String::from_utf8(result.stderr).unwrap();

    assert_eq!(stderr, expected);

    // update files

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

    change_dir_testfile1_content_and_remove_dir_testfile2(path);
    let testfile1_path = format!("{}/dir/testfile1.txt", path);
    let testfile2_path = format!("{}/dir/testfile2.txt", path);
    println!("testfile1: {}", testfile1_path);
    let testfile1_content = fs::read_to_string(testfile1_path.clone()).unwrap();
    assert_eq!(testfile1_content, "Cambio!".to_string());
    assert!(!Path::new(&testfile2_path).exists());

    _ = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_content = fs::read_to_string(testfile1_path).unwrap();
    assert_eq!(testfile1_content, "test".to_string());
    assert!(Path::new(&testfile2_path).exists());

    assert_eq!(stderr, expected);

    _ = fs::remove_dir_all(format!("{}", path));
}
