use core::panic;
use std::{
    fs::{self},
    io::{self},
    path::Path,
    process::Command,
    thread::sleep,
};

// Comando para iniciar daemon
// cargo build; clear; cd ../server_files/; ../../../../../target/debug/git-server
#[test]
#[ignore = "Needs server"]
fn test() {
    let path = "./tests/data/test_delta_objects";

    create_base_scene(path.clone());
    let result = Command::new("git")
        .arg("push")
        .current_dir(path.to_string() + "/repo")
        .output()
        .unwrap();
    println!("stderr: {}", String::from_utf8(result.stderr).unwrap());
    println!("stdout: {}", String::from_utf8(result.stdout).unwrap());

    sleep(std::time::Duration::from_millis(500));
    let result = Command::new("git")
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .arg("repo_copy")
        .current_dir(path)
        .output()
        .unwrap();

    println!("stderr: {}", String::from_utf8(result.stderr).unwrap());
    println!("stdout: {}", String::from_utf8(result.stdout).unwrap());
    sleep(std::time::Duration::from_millis(500));

    let new_content = fs::read_to_string(path.to_string() + "/repo_copy/file1").unwrap();
    let new_content_expected = fs::read_to_string(path.to_string() + "/file2").unwrap();

    assert_eq!(new_content, new_content_expected);

    _ = fs::remove_dir_all(format!("{}/repo", path));
    _ = fs::remove_dir_all(format!("{}/repo_copy", path));
    _ = fs::remove_dir_all(format!("{}/user1_to_send_delta", path));
    _ = fs::remove_dir_all(format!("{}/server_files/repo", path));
    _ = fs::remove_dir_all(format!("{}/server_files/repo_backup", path));
    _ = fs::remove_file(format!("{}/server_files/http-server-logs.log", path));
    _ = fs::remove_file(format!("{}/server_files/tcp-server-logs.log", path));

    _ = fs::remove_file(format!("{}/server_files/daemon.log", path));
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}/server_files/repo", path));
    _ = fs::remove_dir_all(format!("{}/repo", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    copy_dir_all(
        format!("{}/server_files/repo_backup", path),
        format!("{}/server_files/repo", path),
    )
    .unwrap();
    copy_dir_all(
        format!("{}/user1_to_send_delta", path),
        format!("{}/repo", path),
    )
    .unwrap();
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
