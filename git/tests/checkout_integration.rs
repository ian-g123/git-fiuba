use common::aux::create_base_scene;

mod common {
    pub mod aux;
}

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::Command,
    result,
};

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, change_testfile_content,
    create_test_scene_1, create_test_scene_2, get_head,
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

    check_commit(path);

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

#[test]
fn test_ckeckout() {
    let path = "./tests/data/commands/checkout/repo2";

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
    check_commit(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();
    check_commit(path);
    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stderr: {}", stderr);
    println!("stdout: {}", stdout);
    let head_path = format!("{}/.git/HEAD", path);
    let expected = "Switched to branch 'branch1'\n".to_string();

    assert_eq!(expected, stdout);

    let head_ref = fs::read_to_string(head_path.clone()).unwrap();
    let expected = "ref: refs/heads/branch1".to_string();
    assert_eq!(expected, head_ref);

    // Not overlapping changes

    change_dir_testfile1_content_and_remove_dir_testfile2(path); // 1 deleted, 1 modificated, 1 added
    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Status: {}", stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("master")
        .current_dir(path)
        .output()
        .unwrap();
    check_commit(path);

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stderr: {}", stderr);
    println!("stdout: {}", stdout);

    let expected = "M\tdir/testfile1.txt\nD\tdir/testfile2.txt\nA\tdir/testfile3.txt\nSwitched to branch 'master'\n".to_string();

    assert_eq!(expected, stdout);

    let head_ref = fs::read_to_string(head_path.clone()).unwrap();
    let expected = "ref: refs/heads/master".to_string();
    assert_eq!(expected, head_ref);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Status: {}", stdout);

    // Overlaping changes

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Status: {}", stdout);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();
    check_commit(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Status: {}", stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("Checkout -stderr: {}", stderr);
    println!("stdout: {}", stdout);
    check_commit(path);

    assert_eq!(stdout, "Switched to branch 'branch1'\n");

    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"cambio file 3 con overlapping!").unwrap();
    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"cambio file 1 con overlapping!").unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("master")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stderr: {}", stderr);
    println!("stdout: {}", stdout);

    let expected = "error: The following untracked working tree files would be overwritten by checkout:\n\tdir/testfile3.txt\nPlease commit your changes or stash them before you switch branches.\nAborting\nerror: Your local changes to the following files would be overwritten by checkout:\n\tdir/testfile1.txt\nPlease move or remove them before you switch branches.\nAborting\n".to_string();

    assert_eq!(expected, stdout);

    let head_ref = fs::read_to_string(head_path.clone()).unwrap();
    let expected = "ref: refs/heads/branch1".to_string();
    assert_eq!(expected, head_ref);

    let testfile1_path = format!("{}/dir/testfile1.txt", path);
    let testfile2_path = format!("{}/dir/testfile2.txt", path);
    let testfile3_path = format!("{}/dir/testfile3.txt", path);

    println!("testfile1: {}", testfile1_path);
    let testfile1_content = fs::read_to_string(testfile1_path.clone()).unwrap();
    let testfile2_content = fs::read_to_string(testfile2_path.clone()).unwrap();
    let testfile3_content = fs::read_to_string(testfile3_path.clone()).unwrap();

    assert_eq!(
        testfile1_content,
        "cambio file 1 con overlapping!".to_string()
    );

    assert_eq!(testfile2_content, "test".to_string());
    assert_eq!(
        testfile3_content,
        "cambio file 3 con overlapping!".to_string()
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_create_and_checkout() {
    let path = "./tests/data/commands/checkout/repo3";

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
        .arg("branch")
        .arg("base")
        .current_dir(path)
        .output()
        .unwrap();

    change_dir_testfile1_content_and_remove_dir_testfile2(path); // 1 deleted, 1 modificated, 1 added
    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("checkout")
        .arg("-b")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stderr: {}", stderr);
    println!("stdout: {}", stdout);
    let head_path = format!("{}/.git/HEAD", path);
    let expected = "M\tdir/testfile1.txt\nD\tdir/testfile2.txt\nA\tdir/testfile3.txt\nSwitched to branch 'branch1'\n".to_string();

    assert_eq!(expected, stdout);

    let head_ref = fs::read_to_string(head_path.clone()).unwrap();
    let expected = "ref: refs/heads/branch1".to_string();
    assert_eq!(expected, head_ref);

    let testfile1_path = format!("{}/dir/testfile1.txt", path);
    let testfile2_path = format!("{}/dir/testfile2.txt", path);
    let testfile3_path = format!("{}/dir/testfile3.txt", path);

    let testfile1_content = fs::read_to_string(testfile1_path.clone()).unwrap();
    let testfile3_content = fs::read_to_string(testfile3_path.clone()).unwrap();

    assert_eq!(testfile1_content, "Cambio!".to_string());

    assert!(!Path::new(&testfile2_path).exists());
    assert_eq!(testfile3_content, "file 3!".to_string());

    _ = fs::remove_dir_all(format!("{}", path));
}

fn check_commit(path: &str) {
    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    println!("branch ref: {}", branch_ref);
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    println!("Commit hash: {}", commit_hash.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    let output = String::from_utf8(result.stdout).unwrap();
    println!("Commit output: {}", output.clone());
    let output_lines: Vec<&str> = output.split('\n').collect();

    let commit_tree_info: Vec<&str> = output_lines[0].split(" ").collect();
    let commit_tree = commit_tree_info[1];
    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_tree)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!(
        "Commit tree:\nstderr: {}\nstdout: {}\n",
        stderr,
        stdout.clone()
    );

    let output_lines: Vec<&str> = stdout.split('\n').collect();

    let dir_tree_info: Vec<&str> = output_lines[0].split(" ").collect();
    let dir_tree = dir_tree_info[2];

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(dir_tree)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!(
        "Commit tree:\nstderr: {}\nstdout: {}\n",
        stderr,
        stdout.clone()
    );
    /* assert_eq!(
        output_lines[0],
        "tree 43a028a569110ece7d1d1ee46f3d1e50fdcf7946"
    );
    assert!(output_lines[1]
        .to_string()
        .starts_with("author Foo Bar <example@email.org>"));
    assert!(output_lines[1].to_string().ends_with(" -0300"));
    assert!(output_lines[2]
        .to_string()
        .starts_with("committer Foo Bar <example@email.org>"));
    assert!(output_lines[2].to_string().ends_with("-0300"));
    assert_eq!(output_lines[3], ""); */
}
