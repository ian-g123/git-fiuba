use core::panic;
use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    path::Path,
    process::{Child, Command},
};

use git_lib::{file_compressor::extract, join_paths};

#[test]
#[ignore]
fn test_clone() {
    let path = "./tests/data/commands/clone/test1";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone());
    // let mut handle = start_deamon(path);
    // let id = handle.id();
    // println!("ID: {}", id);

    let result = Command::new(git_bin)
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .current_dir(path)
        .output()
        .unwrap();

    compare_files(
        &format!("{}/repo/", path),
        "2a293f24ce241ead407caf5bcd23fcde82c63149",
        &format!("{}/server-files/repo/", path),
        "2a293f24ce241ead407caf5bcd23fcde82c63149",
    );
    compare_files(
        &format!("{}/repo/", path),
        "764df4e2bf5c8afd5ab625cda76bdc30ece1eeef",
        &format!("{}/server-files/repo/", path),
        "764df4e2bf5c8afd5ab625cda76bdc30ece1eeef",
    );

    let ref_path = path.to_owned() + "/repo/.git/refs/remotes/origin/master";
    println!("{}", ref_path);
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    compare_files(
        &format!("{}/repo/", path),
        &commit_hash,
        &format!("{}/server-files/repo/", path),
        &commit_hash,
    );

    let joined_path =
        join_paths!(path.to_owned(), "repo/testfile").expect("No se pudo unir los paths");
    println!("joined_path {}", joined_path);
    let mut file = File::open(joined_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(content, "contenido\n");

    modify_file_and_commit_in_server_repo(&path);

    let result = Command::new("../".to_owned() + git_bin)
        .arg("fetch")
        .current_dir(&format!("{}/repo/", path))
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stdout).unwrap());
    println!("{}", String::from_utf8(result.stderr).unwrap());

    let result = Command::new("../".to_owned() + git_bin)
        .arg("merge")
        .current_dir(&format!("{}/repo/", path))
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());

    let mut file = File::open(path.to_owned() + "/repo/testfile").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(content, "Primera linea\nSeparador\nTercera linea\n");

    panic!("Pausa");

    _ = fs::remove_dir_all(format!("{}", path));
}

fn modify_file_and_commit_in_server_repo(path: &str) {
    let mut file = File::create(path.to_owned() + "/server-files/repo/testfile").unwrap();
    file.write_all(b"Primera linea\nSeparador\nTercera linea\n")
        .unwrap();

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );
}

fn start_deamon(path: &str) -> Child {
    let handle = Command::new("git")
        .arg("daemon")
        .arg("--verbose")
        .arg("--reuseaddr")
        .arg("--enable=receive-pack")
        .arg("--base-path=.")
        .current_dir(path.to_owned() + "/server-files")
        .spawn()
        .expect("No se pudo iniciar el daemon");

    handle
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
    _ = fs::remove_dir_all(format!("{}/repo", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let Ok(_) = fs::create_dir_all(format!("{}/server-files/repo", path)) else {
        panic!("No se pudo crear el directorio")
    };

    assert!(
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );

    let mut file = File::create(path.to_owned() + "/server-files/repo/testfile").unwrap();
    file.write_all(b"contenido\n").unwrap();

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new("touch")
            .arg("git-daemon-export-ok")
            .current_dir(path.to_owned() + "/server-files/repo/.git")
            .status()
            .is_ok(),
        "No se pudo crear el archivo testfile"
    );
}

fn read_file(repo_path: &str, hash_str: &str) -> Result<Vec<u8>, Error> {
    let path = format!(
        "{}.git/objects/{}/{}",
        &repo_path,
        &hash_str[0..2],
        &hash_str[2..]
    );
    let mut file = File::open(&path).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let decompressed_data = extract(&data).unwrap();
    Ok(decompressed_data)
}

fn compare_files(repo_path_1: &str, hash_str_1: &str, repo_path_2: &str, hash_str_2: &str) {
    let file1 = read_file(repo_path_1, hash_str_1).unwrap();
    let file2 = read_file(repo_path_2, hash_str_2).unwrap();
    assert_eq!(file1, file2);
}
