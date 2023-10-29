use core::panic;
use std::{
    fs::{self, File},
    io::{Error, Read},
    path::Path,
    process::{Child, Command},
};

use git_lib::file_compressor::extract;

#[test]
fn test_clone() {
    let path = "./tests/data/commands/clone/test1";

    // _ = fs::remove_dir_all(format!("{}/repo", path));
    // create_base_scene(path.clone());
    // let mut handle = start_deamon(path);
    // let id = handle.id();
    // println!("ID: {}", id);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .current_dir(path)
        .output()
        .unwrap();
    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());

    // match handle.kill() {
    //     Ok(_) => {}
    //     Err(error) => {
    //         panic!("No se pudo matar el proceso {}: {}", id, error);
    //     }
    // }
    // handle.wait().unwrap();

    //check if the files are the same in both directories
    compare_files(
        &format!("{}/repo/", path),
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        &format!("{}/server-files/repo/", path),
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
    );
    compare_files(
        &format!("{}/repo/", path),
        "d58e8558871f7a6001e4cb58b852567ccce91022",
        &format!("{}/server-files/repo/", path),
        "d58e8558871f7a6001e4cb58b852567ccce91022",
    );
    compare_files(
        &format!("{}/repo/", path),
        "c6dc3ef12a2f816f638276a075c9b88c7a458b0a",
        &format!("{}/server-files/repo/", path),
        "c6dc3ef12a2f816f638276a075c9b88c7a458b0a",
    );
    panic!("Pausa");

    _ = fs::remove_dir_all(format!("{}", path));
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
    _ = fs::remove_dir_all(format!("{}", path));
    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    assert!(
        Command::new("mkdir")
            .arg("server-files")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear el directorio repo"
    );

    assert!(
        Command::new("mkdir")
            .arg("repo")
            .current_dir(path.to_owned() + "/server-files")
            .status()
            .is_ok(),
        "No se pudo crear el directorio repo"
    );

    assert!(
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );

    assert!(
        Command::new("touch")
            .arg("testfile")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo crear el archivo testfile"
    );

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
