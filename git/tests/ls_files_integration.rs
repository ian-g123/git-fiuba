use std::{
    fs::{self, File},
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

#[test]
fn test_unmerged() {
    let path = "./tests/data/commands/ls_files/repo2";

    create_test_scene_5(path);

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
    println!("Common commit {}", master_commit);
    let testfile1_hash_common = get_hash("dir/testfile1.txt", path);
    let testfile2_hash_common = get_hash("dir/testfile2.txt", path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("-b")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    println!(
        "Checkout error: {}",
        String::from_utf8(result.stderr).unwrap()
    );
    println!(
        "Checkout stdout: {}",
        String::from_utf8(result.stdout).unwrap()
    );

    change_dir_testfile1_content_and_remove_dir_testfile2_bis(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();
    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let branch1_path = format!("{}/.git/refs/heads/branch1", path);
    let branch1_commit = fs::read_to_string(branch1_path).unwrap();
    println!("Branch1 commit {}", branch1_commit.clone());
    println!(
        "Commit error: {}",
        String::from_utf8(result.stderr).unwrap()
    );
    println!(
        "Commit stdout: {}",
        String::from_utf8(result.stdout).unwrap()
    );

    let testfile1_hash_remote = get_hash("dir/testfile1.txt", path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("master")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stderr).unwrap(), "");

    change_dir_testfile1_testfile2_unmerged(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path.clone()).unwrap();
    println!("Master commit {}", master_commit);

    println!(
        "Commit error: {}",
        String::from_utf8(result.stderr).unwrap()
    );
    println!(
        "Commit stdout: {}",
        String::from_utf8(result.stdout).unwrap()
    );

    let testfile1_hash_head = get_hash("dir/testfile1.txt", path);
    let testfile2_hash_head = get_hash("dir/testfile2.txt", path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("merge")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Stdout: {}", stdout);

    let master_commit = fs::read_to_string(master_path.clone()).unwrap();
    println!("Master commit {}", master_commit);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Stdout: {}", stdout);

    let expected = "On branch master\nYou have unmerged paths.\n  (fix conflicts and run \"git commit\")\n  (use \"git merge --abort\" to abort the merge)\n\nUnmerged paths:\n  (use \"git add/rm <file>...\" as appropriate to mark resolution)\n\tboth modified:   dir/testfile1.txt\n\tdeleted by them:   dir/testfile2.txt\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\tdir/dir1/\n\tdir/testfile3.txt\n\tdir/testfile4.txt\n\ttestfile.txt\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\"\n";
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-u")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("100644 {testfile1_hash_common} 1\tdir/testfile1.txt\n100644 {testfile1_hash_head} 2\tdir/testfile1.txt\n100644 {testfile1_hash_remote} 3\tdir/testfile1.txt\n100644 {testfile2_hash_common} 1\tdir/testfile2.txt\n100644 {testfile2_hash_head} 2\tdir/testfile2.txt\n");
    assert_eq!(stdout, expected);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_modified() {
    let path = "./tests/data/commands/ls_files/repo3";

    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
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

    change_dir_testfile1_content_and_remove_dir_testfile2_bis(path);
    change_testfile_content(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/testfile1.txt\ndir/testfile2.txt\n";

    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/testfile1.txt\ndir/testfile2.txt\ndir/testfile2.txt\n";

    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .arg("-c")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/dir1/testfile5.txt\ndir/testfile1.txt\ndir/testfile1.txt\ndir/testfile2.txt\ndir/testfile2.txt\ndir/testfile3.txt\ndir/testfile4.txt\ntestfile.txt\n";

    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .arg("-o")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/dir1/testfile6.txt\ndir/testfile1.txt\ndir/testfile2.txt\n";

    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/testfile1.txt\n";

    assert_eq!(stdout, expected);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-m")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "";

    assert_eq!(stdout, expected);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_files() {
    let path = "./tests/data/commands/ls_files/repo4";

    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
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

    change_dir_testfile1_content_and_remove_dir_testfile2_bis(path);
    change_testfile_content(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "dir/testfile1.txt\n";

    assert_eq!(stdout, expected);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-files")
        .arg("-d")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "";

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

fn change_dir_testfile1_testfile2_unmerged(path: &str) {
    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"linea 1\nlinea 5\nlinea 3").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile2.txt").unwrap();
    file.write_all(b"Conflicto porque en la otra rama se borro y en esta se modifico!")
        .unwrap();
}

pub fn change_dir_testfile1_content_and_remove_dir_testfile2_bis(path: &str) {
    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"linea 1\nlinea 6\nlinea 3").unwrap();
    _ = fs::remove_file(path.to_string() + "/dir/testfile2.txt").unwrap();
}

pub fn change_testfile_content_bis(path: &str) {
    let mut file = File::create(path.to_owned() + "/testfile.txt").unwrap();
    file.write_all(b"cambio x2").unwrap();
}
