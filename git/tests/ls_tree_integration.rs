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
fn test_tree() {
    let path = "./tests/data/commands/ls_tree/repo1";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    let tree = get_commit_tree(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected =
        format!("040000 tree {hash_dir}\tdir\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("040000 tree {hash_dir}\tdir\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-d")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("040000 tree {hash_dir}\tdir\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("--name-only")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("dir\ntestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .arg("--name-only")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("dir\ndir/dir1\ndir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile1.txt\ndir/testfile2.txt\ndir/testfile3.txt\ndir/testfile4.txt\ntestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .arg("--name-only")
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("dir\ndir/dir1\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .arg("--name-only")
        .arg("-l")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("dir\ndir/dir1\ndir/dir1/testfile5.txt\ndir/dir1/testfile6.txt\ndir/testfile1.txt\ndir/testfile2.txt\ndir/testfile3.txt\ndir/testfile4.txt\ntestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(tree.clone())
        .arg("-r")
        .arg("-t")
        .arg("-l")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("040000 tree {hash_dir}       -\tdir\n040000 tree {hash_dir1}       -\tdir/dir1\n100644 blob {hash_testfile5}       7\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}       7\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}      23\tdir/testfile1.txt\n100644 blob {hash_testfile2}       4\tdir/testfile2.txt\n100644 blob {hash_testfile3}       7\tdir/testfile3.txt\n100644 blob {hash_testfile4}       7\tdir/testfile4.txt\n100644 blob {hash_testfile}       9\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_commit() {
    let path = "./tests/data/commands/ls_tree/repo2";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    let commit = get_commit(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg(commit.clone())
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_head() {
    let path = "./tests/data/commands/ls_tree/repo3";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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
        .arg("ls-tree")
        .arg("HEAD")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_local_branch() {
    let path = "./tests/data/commands/ls_tree/repo4";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("b1")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("heads/b1")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_remote_branch() {
    let path = "./tests/data/commands/ls_tree/repo5";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    let commit = get_commit(path);

    let Ok(_) = fs::create_dir_all(path.to_owned() + "/.git/refs/remotes/origin/") else {
        panic!("No se pudo crear el directorio")
    };
    let mut file = File::create(path.to_owned() + "/.git/refs/remotes/origin/b1").unwrap();
    file.write_all(commit.as_bytes()).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("origin/b1")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_tag() {
    let path = "./tests/data/commands/ls_tree/repo6";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    let tree = get_commit_tree(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("tag1")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);
    let hash_testfile2 = get_hash("dir/testfile2.txt", path);
    let hash_testfile3 = get_hash("dir/testfile3.txt", path);
    let hash_testfile4 = get_hash("dir/testfile4.txt", path);
    let hash_testfile5 = get_hash("dir/dir1/testfile5.txt", path);
    let hash_testfile6 = get_hash("dir/dir1/testfile6.txt", path);
    let hash_testfile = get_hash("testfile.txt", path);
    let hash_dir = "d7ed8c1d109986c59f37c1a54cc3b40af916b41f";
    let hash_dir1 = "c33ddf0cee67a1536f0cad727e73103cb0f15b30";

    let expected = format!("040000 tree {hash_dir}\tdir\n040000 tree {hash_dir1}\tdir/dir1\n100644 blob {hash_testfile5}\tdir/dir1/testfile5.txt\n100644 blob {hash_testfile6}\tdir/dir1/testfile6.txt\n100644 blob {hash_testfile1}\tdir/testfile1.txt\n100644 blob {hash_testfile2}\tdir/testfile2.txt\n100644 blob {hash_testfile3}\tdir/testfile3.txt\n100644 blob {hash_testfile4}\tdir/testfile4.txt\n100644 blob {hash_testfile}\ttestfile.txt\n");

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("tag1")
        .arg("-a")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    println!("Stdout: {}", String::from_utf8(result.stdout).unwrap());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("tags/tag2")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag3")
        .arg(tree)
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    println!("Stdout: {}", String::from_utf8(result.stdout).unwrap());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("tag3")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    println!("Stderr: {}", String::from_utf8(result.stderr).unwrap());

    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_error() {
    let path = "./tests/data/commands/ls_tree/repo7";
    create_test_scene_5(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .arg("dir/testfile3.txt")
        .arg("dir/testfile4.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("dir/dir1/testfile6.txt")
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

    let hash_testfile1 = get_hash("dir/testfile1.txt", path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag1")
        .arg("-a")
        .arg("-m")
        .arg("message")
        .arg(hash_testfile1)
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .arg("tag1")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("ls-tree")
        .arg("tag2")
        .arg("-r")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "fatal: not a tree object\n";

    assert_eq!(expected, stderr);
    assert_eq!("", stdout);

    _ = std::fs::remove_dir_all(format!("{}", path));
}

fn get_commit(path: &str) -> String {
    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    fs::read_to_string(ref_path).unwrap()
}

fn get_commit_tree(path: &str) -> String {
    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    let output = String::from_utf8(result.stdout).unwrap();
    let output_lines: Vec<&str> = output.split('\n').collect();

    let commit_tree_info: Vec<&str> = output_lines[0].split(" ").collect();
    commit_tree_info[1].to_string()
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
