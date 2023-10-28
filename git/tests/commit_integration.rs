use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

use crate::common::aux::{
    change_dir_testfile1_content, change_dir_testfile1_content_and_remove_dir_testfile2,
    create_test_scene_1, create_test_scene_2, create_test_scene_3,
};

mod common {
    pub mod aux;
}

/// Prueba que se pueda commitear un solo archivo.
#[test]
fn test_single_file() {
    let path = "./tests/data/commands/commit/repo1";
    create_test_scene_1(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("43a028a569110ece7d1d1ee46f3d1e50fdcf7946")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 30d74d258442c7c65512eafab474568dd706c430    testfile.txt\n"
    );

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
    assert_eq!(
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
    assert_eq!(output_lines[3], "");
    assert_eq!(output_lines[4], "message");

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba que se puedan commitear únicamente los cambios agregados al staging area.
#[test]
fn test_commit_some_changes() {
    let path = "./tests/data/commands/commit/repo2";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

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

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("4b86ab26030de52e745b22cbf82d372500708089")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree 761f3460563f71d56a3509a761d9c531423c52b8    dir\n"
    );

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("761f3460563f71d56a3509a761d9c531423c52b8")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 30d74d258442c7c65512eafab474568dd706c430    testfile1.txt\n"
    );
    let output_lines: Vec<&str> = output.split('\n').collect();
    assert_eq!(
        output_lines[0],
        "tree 4b86ab26030de52e745b22cbf82d372500708089"
    );
    assert!(output_lines[1]
        .to_string()
        .starts_with("author Foo Bar <example@email.org>"));
    assert!(output_lines[1].to_string().ends_with(" -0300"));
    assert!(output_lines[2]
        .to_string()
        .starts_with("committer Foo Bar <example@email.org>"));
    assert!(output_lines[2].to_string().ends_with("-0300"));
    assert_eq!(output_lines[3], "");
    assert_eq!(output_lines[4], "message");

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba el correcto funcionamiento del flag 'all'.
#[test]
fn test_flag_all() {
    let path = "./tests/data/commands/commit/repo4";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    change_dir_testfile1_content(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(testfile1_hash.trim())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "Cambio!\n");

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();

    let work_tree_hash = output.lines().next().unwrap().split_once(' ').unwrap().1;

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(work_tree_hash)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree ed3adf248ce4d5fe5d89ac33798e4c92e3693da9    dir\n"
    );

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("ed3adf248ce4d5fe5d89ac33798e4c92e3693da9")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 9d1bdbbe7e41c96f5eb2231cc98240845610f183    testfile1.txt\n"
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba el correcto funcionamiento del flag 'all' cuando hay archivos eliminados en el
/// working tree.
#[test]
fn test_flag_all_with_deleted_files() {
    let path = "./tests/data/commands/commit/repo3";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(testfile1_hash.trim())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "Cambio!\n");

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
    println!("Output: \n{}", output);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("8403dbf1a48258117de1aff300010280ce9d4790")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree ed3adf248ce4d5fe5d89ac33798e4c92e3693da9    dir\n"
    );

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("ed3adf248ce4d5fe5d89ac33798e4c92e3693da9")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 9d1bdbbe7e41c96f5eb2231cc98240845610f183    testfile1.txt\n"
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba el correcto funcionamiento del flag 'C'.
#[test]
fn test_reuse_message() {
    let path = "./tests/data/commands/commit/repo6";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let hash1 = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();

    let output_lines: Vec<&str> = output.split('\n').collect();
    let tree_hash = output_lines[0];
    let author = output_lines[1];
    let commiter = output_lines[2];
    let message = output_lines[4];

    change_dir_testfile1_content(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let hash2 = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-C")
        .arg(commit_hash)
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

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

    assert_ne!(tree_hash, output_lines[0]);
    assert_eq!(author, output_lines[2]);
    assert_eq!(commiter, output_lines[3]);
    assert_eq!(message, output_lines[5]);

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba que se puedan agregar al staging area los archivos pasados al comando Commit.
#[test]
fn test_commit_paths() {
    let path = "./tests/data/commands/commit/repo7";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile2_hash = String::from_utf8(result.stdout).unwrap();

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    let output_lines: Vec<&str> = output.split('\n').collect();
    let tree_hash = output_lines[0];
    let tree_hash: Vec<&str> = tree_hash.split(" ").collect();
    println!("Tree hash: \n {}", tree_hash[1].trim());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(tree_hash[1].trim())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("d939d691de20dfedb6f26862d09aec381eb564cd")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("f10179baba8f747e1ebd03285670677fbcad7249")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree b97187ddd9b15b87e689b9e6eb5358db7951b9a2    dir\n"
    );

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("b97187ddd9b15b87e689b9e6eb5358db7951b9a2")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = format!(
        "100644 blob {}    testfile1.txt\n100644 blob {}    testfile2.txt\n",
        testfile1_hash.trim(),
        testfile2_hash.trim()
    );

    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba que no se puedan agregar al staging area los archivos pasados al comando Commit
/// que no son registrados por git.
#[test]
fn test_commit_paths_fails() {
    let path = "./tests/data/commands/commit/repo8";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());
    let expected = "error: pathspec 'dir/testfile3.txt' did not match any file(s) known to git\n";
    assert_eq!(String::from_utf8(result.stderr).unwrap(), expected);

    _ = fs::remove_dir_all(format!("{}", path));
}
