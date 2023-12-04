use core::panic;
use std::{
    fs::{self, File},
    io::{self, Error, Read, Write},
    path::Path,
    process::{Child, Command},
};

use git_lib::{file_compressor::extract, join_paths};

// Comando para iniciar daemon
// cd ../server-files/; clear; git daemon --verbose --reuseaddr --enable=receive-pack --informative-errors --base-path=. .
#[test]
#[ignore = "Needs server"]
fn test() {
    let path = "./tests/data/commands/labdeltaclient";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone());
    let result = Command::new(git_bin)
        .arg("fetch")
        .current_dir(path.to_string() + "/repo")
        .output()
        .unwrap();
    println!("stderr: {}", String::from_utf8(result.stderr).unwrap());
    println!("stdout: {}", String::from_utf8(result.stdout).unwrap());

    let result = Command::new(git_bin)
        .arg("merge")
        .current_dir(path.to_string() + "/repo")
        .output()
        .unwrap();
    println!("stderr: {}", String::from_utf8(result.stderr).unwrap());
    println!("stdout: {}", String::from_utf8(result.stdout).unwrap());

    let new_content = fs::read_to_string(path.to_string() + "/repo/file1").unwrap();
    let new_content_expected = fs::read_to_string(path.to_string() + "/file2").unwrap();

    assert_eq!(new_content, new_content_expected);
    _ = fs::remove_dir_all(format!("{}/repo", path));
    _ = fs::remove_dir_all(format!("{}/user2_to_recieve_delta", path));
    _ = fs::remove_dir_all(format!("{}/server_files/repo", path));
    _ = fs::remove_dir_all(format!("{}/server_files/repo_with_two_commits", path));

    _ = fs::remove_file(format!("{}/server_files/daemon.log", path));
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}/server_files/repo", path));
    _ = fs::remove_dir_all(format!("{}/repo", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    copy_dir_all(
        format!("{}/server_files/repo_with_two_commits", path),
        format!("{}/server_files/repo", path),
    )
    .unwrap();
    copy_dir_all(
        format!("{}/user2_to_recieve_delta", path),
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
