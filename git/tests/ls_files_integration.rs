use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    process::Command,
};

use common::aux::create_base_scene;

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, change_test_scene_4,
    create_test_scene_2, create_test_scene_4, create_test_scene_5,
};

mod common {
    pub mod aux;
}

#[test]
fn test_general() {
    let path = "./tests/data/commands/ls_files/repo1";

    create_test_scene_5(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";
    assert_eq!(stdout, expected);

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

    // add

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let expected = "dir/testfile1.txt\ndir/testfile3.txt\n";

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-o")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "dir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile2.txt\ndir/testfile4.txt\ntestfile.txt\n";
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let expected = format!("100644 {hash_testfile1} 0\tdir/testfile1.txt\n100644 {hash_testfile3} 0\tdir/testfile3.txt\n");

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-s")
        .arg("-o")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let expected = format!("dir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile2.txt\ndir/testfile4.txt\ntestfile.txt\n100644 {hash_testfile1} 0\tdir/testfile1.txt\n100644 {hash_testfile3} 0\tdir/testfile3.txt\n");

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    // change + delete
    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    change_dir_testfile1_content_and_remove_dir_testfile2(path);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .current_dir(path)
        .output()
        .unwrap();
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let expected = "dir/testfile1.txt\ndir/testfile2.txt\ndir/testfile3.txt\n";

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-o")
        .current_dir(path)
        .output()
        .unwrap();
    let expected =
        "dir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile4.txt\ntestfile.txt\n";
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "dir/testfile1.txt\ndir/testfile2.txt\n";
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "dir/testfile2.txt\n";
    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout, expected);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

fn get_hash(file: &str, path: &str) -> String {
    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg(file)
        .current_dir(path)
        .output()
        .unwrap();
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}
