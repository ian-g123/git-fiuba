

mod common {
    pub mod aux;
}

use std::{
    fs::{self, File},
    io::{Write},
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

    // crea una rama a partir de un path (no name)

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("dir/branch4")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch4_path = format!("{}/.git/refs/heads/dir/branch4", path);
    println!("Branch4: {}", branch4_path);

    assert!(Path::new(&branch4_path).exists());

    // crear a partir de una remota

    create_remote_files(path, "origin/");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch5")
        .arg("remotes/origin/remote1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch5_path = format!("{}/.git/refs/heads/branch5", path);
    println!("Branch5: {}", branch5_path);

    assert!(Path::new(&branch5_path).exists());

    _ = fs::remove_dir_all(path.to_string());
}

#[test]
fn test_rename_branch() {
    let path = "./tests/data/commands/branch/repo2";

    create_test_scene_1(path);
    // cambiar nombre de head

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "error: refname refs/heads/master\nfatal: Branch rename failed\n".to_string();
    assert_eq!(expected, stderr);

    // cambiar nombre de HEAD (existe)

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

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path.clone()).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let branch1_path = format!("{}/.git/refs/heads/branch1", path);

    assert!(Path::new(&branch1_path).exists());
    assert!(!Path::new(&master_path).exists());

    let branch1_commit = fs::read_to_string(branch1_path).unwrap();

    assert_eq!(master_commit, branch1_commit);

    // cambiar old por new falla porque old no existe.

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("old")
        .arg("new")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "error: refname refs/heads/old\nfatal: Branch rename failed\n".to_string();

    assert_eq!(stderr, expected);

    // cambiar old por new falla porque new existe.

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch2")
        .current_dir(path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("stderr: {}", stderr);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("branch2")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "fatal: A branch named 'branch1' already exists\n".to_string();

    assert_eq!(stderr, expected);

    // cambiar nombre old por new
    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("branch2")
        .arg("branch3")
        .current_dir(path)
        .output()
        .unwrap();

    let _stderr = String::from_utf8(result.stderr).unwrap();

    let branch2_path = format!("{}/.git/refs/heads/branch2", path);
    let branch3_path = format!("{}/.git/refs/heads/branch3", path);

    assert!(Path::new(&branch3_path).exists());
    assert!(!Path::new(&branch2_path).exists());

    // old está dentro de un directorio
    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("dir/branch4")
        .current_dir(path)
        .output()
        .unwrap();

    let branch4_path = format!("{}/.git/refs/heads/dir/branch4", path);
    let branch4_commit = fs::read_to_string(branch4_path.clone()).unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("dir/branch4")
        .arg("branch5")
        .current_dir(path)
        .output()
        .unwrap();

    let _stderr = String::from_utf8(result.stderr).unwrap();

    let branch5_path = format!("{}/.git/refs/heads/branch5", path);
    let branch5_commit = fs::read_to_string(branch5_path.clone()).unwrap();

    assert!(Path::new(&branch5_path).exists());
    assert!(!Path::new(&branch4_path).exists());
    assert_eq!(branch4_commit, branch5_commit);
    // new está dentro de un directorio

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-m")
        .arg("branch5")
        .arg("dir/branch4")
        .current_dir(path)
        .output()
        .unwrap();

    let _stderr = String::from_utf8(result.stderr).unwrap();

    let branch4_commit = fs::read_to_string(branch4_path.clone()).unwrap();

    assert!(Path::new(&branch4_path).exists());
    assert!(!Path::new(&branch5_path).exists());
    assert_eq!(branch4_commit, branch5_commit);

    _ = fs::remove_dir_all(path.to_string());
}

#[test]
fn test_delete_branch() {
    let path = "./tests/data/commands/branch/repo3";

    create_test_scene_1(path);
    // delete with no args

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-D")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let expected = "fatal: branch name required\n".to_string();
    let stdout = String::from_utf8(result.stdout).unwrap();
    println!("stdout: {}", stdout);
    assert_eq!(expected, stderr);

    // delete local branches 2 existen, 1 no.

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

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit = fs::read_to_string(master_path).unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch2")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .arg("-D")
        .arg("branch2")
        .arg("no-existe")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = format!("error: branch 'no-existe' not found.\nDeleted branch branch1 (was {}).\nDeleted branch branch2 (was {}).\n", &master_commit[..7],&master_commit[..7]);

    let branch1_path = format!("{}/.git/refs/heads/branch1", path);
    let branch2_path = format!("{}/.git/refs/heads/branch2", path);

    assert!(!Path::new(&branch1_path).exists());
    assert!(!Path::new(&branch2_path).exists());

    assert_eq!(stdout, expected);

    // delete remote branches (solo en el repo local)

    create_remote_files(path, "origin/");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("origin/remote1")
        .arg("origin/dir/remote3")
        .arg("remote3")
        .arg("-D")
        .arg("-r")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = format!("error: remote-tracking branch 'remote3' not found.\nDeleted remote-tracking branch origin/remote1 (was {}).\nDeleted remote-tracking branch origin/dir/remote3 (was {}).\n", "aaaaaaa", "aaaaaaa");

    let remote1_path = format!("{}/.git/refs/remotes/remote1", path);
    let remote3_path = format!("{}/.git/refs/remotes/dir/remote3", path);

    assert!(!Path::new(&remote1_path).exists());
    assert!(!Path::new(&remote3_path).exists());

    assert_eq!(stdout, expected);

    _ = fs::remove_dir_all(path.to_string());
}

#[test]
fn test_show_branch() {
    let path = "./tests/data/commands/branch/repo4";

    create_test_scene_1(path);
    // show local.

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

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch2")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "  branch1\n  branch2\n* master\n";

    assert_eq!(stdout, expected);

    // show remote

    create_remote_files(path, "origin/");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-r")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "  origin/dir/remote3\n  origin/remote1\n  origin/remote2\n";
    assert_eq!(stdout, expected);

    // show all

    create_remote_files(path, "origin/");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("-r")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = "  branch1\n  branch2\n* master\n  remotes/origin/dir/remote3\n  remotes/origin/remote1\n  remotes/origin/remote2\n".to_string();
    assert_eq!(stdout, expected);

    _ = fs::remove_dir_all(path.to_string());
}

fn create_remote_files(path: &str, remote: &str) {
    let dir = path.to_string() + "/.git/refs/remotes/" + remote;
    fs::create_dir_all(dir.clone()).unwrap();
    let mut file = File::create(dir.clone() + "remote1").unwrap();
    file.write_all(b"aaaaaaaaaa").unwrap();
    let mut file = File::create(dir.clone() + "remote2").unwrap();
    file.write_all(b"aaaaaaaaaa").unwrap();

    fs::create_dir_all(dir.clone() + "dir").unwrap();
    let mut file = File::create(dir + "dir/remote3").unwrap();
    file.write_all(b"aaaaaaaaaa").unwrap();
}
