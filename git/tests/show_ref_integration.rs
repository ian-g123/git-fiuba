use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, create_test_scene_2,
};

mod common {
    pub mod aux;
}

#[test]
fn test_show_all_refs() {
    let path = "./tests/data/commands/show_ref/repo1";
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

    let master_path = format!("{}/.git/refs/heads/master", path);
    let master_commit1 = fs::read_to_string(master_path.clone()).unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("-m")
        .arg("message")
        .arg("tag1")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    let tag1_path = format!("{}/.git/refs/tags/tag1", path);
    let tag1_object = fs::read_to_string(tag1_path.clone()).unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("tag2")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch1")
        .current_dir(path)
        .output()
        .unwrap();

    create_remote_files(path, "origin/");
    change_dir_testfile1_content_and_remove_dir_testfile2(path);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("-a")
        .current_dir(path)
        .output()
        .unwrap();

    let master_commit2 = fs::read_to_string(master_path.clone()).unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("branch2")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} refs/heads/branch1\n{} refs/heads/branch2\n{} refs/heads/master\nadb820486b2b81d06d53eb4c6002963e97f3ba7c refs/remotes/origin/dir/remote3\n21b09a47871263c6178edf413e11268c612f266e refs/remotes/origin/remote1\n7181574aad2004f6a546f07186560f7ab0a00fee refs/remotes/origin/remote2\n{} refs/tags/tag1\n{} refs/tags/tag2\n", master_commit1, master_commit2, master_commit2, tag1_object, master_commit1);

    assert_eq!(expected, stdout);

    // + HEAD

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("--head")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} HEAD\n{} refs/heads/branch1\n{} refs/heads/branch2\n{} refs/heads/master\nadb820486b2b81d06d53eb4c6002963e97f3ba7c refs/remotes/origin/dir/remote3\n21b09a47871263c6178edf413e11268c612f266e refs/remotes/origin/remote1\n7181574aad2004f6a546f07186560f7ab0a00fee refs/remotes/origin/remote2\n{} refs/tags/tag1\n{} refs/tags/tag2\n", master_commit2,master_commit1, master_commit2, master_commit2, tag1_object, master_commit1);

    assert_eq!(expected, stdout);

    // --heads

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("--heads")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!(
        "{} refs/heads/branch1\n{} refs/heads/branch2\n{} refs/heads/master\n",
        master_commit1, master_commit2, master_commit2
    );

    assert_eq!(expected, stdout);

    // --tags

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("--tags")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!(
        "{} refs/tags/tag1\n{} refs/tags/tag2\n",
        tag1_object, master_commit1
    );

    assert_eq!(expected, stdout);

    // --heads + --tags

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("--heads")
        .arg("--tags")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} refs/heads/branch1\n{} refs/heads/branch2\n{} refs/heads/master\n{} refs/tags/tag1\n{} refs/tags/tag2\n",master_commit1, master_commit2, master_commit2, tag1_object, master_commit1);

    assert_eq!(expected, stdout);

    // dereference
    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} refs/heads/branch1\n{} refs/heads/branch2\n{} refs/heads/master\nadb820486b2b81d06d53eb4c6002963e97f3ba7c refs/remotes/origin/dir/remote3\n21b09a47871263c6178edf413e11268c612f266e refs/remotes/origin/remote1\n7181574aad2004f6a546f07186560f7ab0a00fee refs/remotes/origin/remote2\n{} refs/tags/tag1\n{} refs/tags/tag1{}\n{} refs/tags/tag2\n", master_commit1, master_commit2, master_commit2, tag1_object,master_commit1, "^{}", master_commit1);

    assert_eq!(expected, stdout);

    //hash
    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{}\n{}\n{}\nadb820486b2b81d06d53eb4c6002963e97f3ba7c\n21b09a47871263c6178edf413e11268c612f266e\n7181574aad2004f6a546f07186560f7ab0a00fee\n{}\n{} refs/tags/tag1{}\n{}\n", master_commit1, master_commit2, master_commit2, tag1_object,master_commit1, "^{}", master_commit1);

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .arg("--hash")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    assert_eq!(expected, stdout);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .arg("--hash=0")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .arg("--hash=5")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!(
        "{}\n{}\n{}\nadb82\n21b09\n71815\n{}\n{} refs/tags/tag1{}\n{}\n",
        &master_commit1[..5],
        &master_commit2[..5],
        &master_commit2[..5],
        &tag1_object[..5],
        &master_commit1[..5],
        "^{}",
        &master_commit1[..5]
    );

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("-d")
        .arg("--hash=-1")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!(
        "{}\n{}\n{}\nadb8\n21b0\n7181\n{}\n{} refs/tags/tag1{}\n{}\n",
        &master_commit1[..4],
        &master_commit2[..4],
        &master_commit2[..4],
        &tag1_object[..4],
        &master_commit1[..4],
        "^{}",
        &master_commit1[..4]
    );

    assert_eq!(expected, stdout);

    // refs

    _ = Command::new("../../../../../../target/debug/git")
        .arg("branch")
        .arg("dir/remote3")
        .current_dir(path)
        .output()
        .unwrap();

    _ = Command::new("../../../../../../target/debug/git")
        .arg("tag")
        .arg("remote3")
        .arg(master_commit1.clone())
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("remote3")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} refs/heads/dir/remote3\nadb820486b2b81d06d53eb4c6002963e97f3ba7c refs/remotes/origin/dir/remote3\n{} refs/tags/remote3\n", master_commit2, master_commit1);

    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("dir/remote3")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = format!("{} refs/heads/dir/remote3\nadb820486b2b81d06d53eb4c6002963e97f3ba7c refs/remotes/origin/dir/remote3\n", master_commit2);

    assert_eq!(expected, stdout);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("show-ref")
        .arg("mote3")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    println!("Stderr: {}", stderr);
    let stdout = String::from_utf8(result.stdout).unwrap();

    let expected = String::new();
    assert_eq!(expected, stdout);

    _ = std::fs::remove_dir_all(path);
}

fn create_remote_files(path: &str, remote: &str) {
    let dir = path.to_string() + "/.git/refs/remotes/" + remote;
    fs::create_dir_all(dir.clone()).unwrap();
    let mut file = File::create(dir.clone() + "remote1").unwrap();
    file.write_all(b"21b09a47871263c6178edf413e11268c612f266e")
        .unwrap();
    let mut file = File::create(dir.clone() + "remote2").unwrap();
    file.write_all(b"7181574aad2004f6a546f07186560f7ab0a00fee")
        .unwrap();

    fs::create_dir_all(dir.clone() + "dir").unwrap();
    let mut file = File::create(dir + "dir/remote3").unwrap();
    file.write_all(b"adb820486b2b81d06d53eb4c6002963e97f3ba7c")
        .unwrap();
}
