use std::{
    fs::{self, File, OpenOptions},
    io::{Error, Read, Write},
    process::Command,
};

use common::aux::create_base_scene;

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, change_test_scene_4,
    change_testfile_content, create_test_scene_2, create_test_scene_4, create_test_scene_5,
};

mod common {
    pub mod aux;
}

#[test]
fn test_comment() {
    let path = "./tests/data/commands/check_ignore/repo1";
    create_check_ignore_scene(path);

    write_to_gitignore(path, false, "# *.txt\n");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!(expected, stdout);
    assert_eq!(expected, stderr);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .arg("-n")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir/testfile1.txt\n";

    assert_eq!(expected, stdout);
    assert_eq!("", stderr);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_gitignore_root_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo2";
    create_check_ignore_scene(path);

    write_to_gitignore(path, false, "# pattern\n*.txt\n");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".gitignore:2:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("--non-matching")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_gitignore_next_level_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo3";
    create_check_ignore_scene(path);

    write_to_gitignore(path, true, "# pattern\n*.txt\n");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/.gitignore:2:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_exclude_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo4";
    create_check_ignore_scene(path);

    write_to_exclude(path, "# pattern\n*.txt\n");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".git/info/exclude:8:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("--non-matching")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_check_ignore_scene(path: &str) {
    create_base_scene(path);
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };

    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"file 1!").unwrap();

    _ = File::create(path.to_owned() + "/.gitignore").unwrap();

    _ = File::create(path.to_owned() + "/dir/.gitignore").unwrap();
}

fn write_to_gitignore(path: &str, dir: bool, content: &str) {
    let path = if dir {
        path.to_owned() + "/dir/.gitignore"
    } else {
        path.to_owned() + "/.gitignore"
    };
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn write_to_exclude(path: &str, content: &str) {
    let path = path.to_owned() + "/.git/info/exclude";
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}
