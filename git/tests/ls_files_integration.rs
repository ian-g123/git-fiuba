use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    process::Command,
};

use common::aux::create_base_scene;

use crate::common::aux::{
    change_test_scene_4, create_test_scene_2, create_test_scene_4, create_test_scene_5,
};

mod common {
    pub mod aux;
}

#[test]
fn test_cached() {
    let path = "./tests/data/commands/ls_files/repo1";

    create_test_scene_5(path);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-o")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "dir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile1.txt\ndir/testfile2.txt\ndir/testfile3.txt\ndir/testfile4.txt\ntestfile.txt\n";
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    _ = std::fs::remove_dir_all(format!("{}", path));
}
