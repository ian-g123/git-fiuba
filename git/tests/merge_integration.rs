use core::panic;
use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    process::{Child, Command},
};

use git_lib::{file_compressor::extract, join_paths};

#[test]
fn test_merge() {
    let path = "./tests/data/commands/merge/test1";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone(), git_bin.clone());

    modify_file_and_commit_in_both_repos_not_overlaping_files(&path, git_bin);

    let _result = Command::new(git_bin)
        .arg("merge")
        .arg("rama1")
        .current_dir(path)
        .output()
        .unwrap();

    let mut file = File::open(path.to_owned() + "/file-remote").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(content, "Contenido remoto\n");

    let mut file = File::open(path.to_owned() + "/file-local").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(content, "Contenido local\n");

    modify_file_and_commit_in_both_repos_none_overlaping_lines(&path, git_bin);

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("rama2")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());

    let mut file = File::open(path.to_owned() + "/testfile").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(
        content,
        "Primera linea modificada en servidor\nSeparador\nTercera linea modificada en local\n"
    );

    modify_file_and_commit_in_both_repos_overlaping_changes(&path, git_bin);

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("rama3")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());

    let mut file = File::open(path.to_owned() + "/testfile").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(
        content,
        "Primera linea modificada en servidor de nuevo\nSeparador\n<<<<<<< HEAD\nTercera linea modificada en local\n=======\nTercera linea modificada en servidor\n>>>>>>> origin\n"
    );

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("--continue")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        "error: Committing is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: Exiting because of an unresolved conflict.\n"
    );

    // Falta chequear commit
    panic!("PAUSA");

    let _result = Command::new(git_bin)
        .arg("add")
        .arg("testfile")
        .current_dir(path)
        .output()
        .unwrap();

    let _result = Command::new(git_bin)
        .arg("merge")
        .arg("--continue")
        .current_dir(path)
        .output()
        .unwrap();

    // panic!("STOP");
    _ = fs::remove_dir_all(format!("{}", path));
}

fn modify_file_and_commit_in_both_repos_none_overlaping_lines(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("branch")
            .arg("rama2")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la rama"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(b"Primera linea modificada en rama2\nSeparador\nTercera linea\n")
        .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("modificacion_servidor_not_overlaping")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la rama"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(b"Primera linea\nSeparador\nTercera linea modificada en master\n")
        .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned())
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_not_overlaping")
        .current_dir(path.to_owned())
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());
}

fn modify_file_and_commit_in_both_repos_overlaping_changes(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("branch")
            .arg("rama3")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la rama"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(
        b"Primera linea modificada en servidor de nuevo\nSeparador\nTercera linea modificada en servidor\n",
    )
    .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("modificacion_servidor_overlaping")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a master"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(
        b"Primera linea modificada en servidor\nSeparador\nTercera linea modificada en local\n",
    )
    .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned())
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let _result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_overlaping")
        .current_dir(path.to_owned())
        .output()
        .unwrap();
}

fn modify_file_and_commit_in_both_repos_not_overlaping_files(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("branch")
            .arg("rama1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la rama"
    );

    let mut file = File::create(path.to_owned() + "/file-remote").unwrap();
    file.write_all(b"Contenido remoto\n").unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("file-remote")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo file-remote"
    );

    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("modificacion_rama_no_overlaping_files")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a master"
    );

    let mut file = File::create(path.to_owned() + "/file-local").unwrap();
    file.write_all(b"Contenido local\n").unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("file-local")
            .current_dir(path.to_owned())
            .status()
            .is_ok(),
        "No se pudo agregar el archivo file-local"
    );

    let result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_no_overlaping_files")
        .current_dir(path.to_owned())
        .output()
        .unwrap();
}

fn modify_file_and_commit_in_server_repo(path: &str, git_bin: &str) {
    let mut file = File::create(path.to_owned() + "/server-files/repo/testfile").unwrap();
    file.write_all(b"Primera linea\nSeparador\nTercera linea\n")
        .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );
}

fn create_base_scene(path: &str, git_bin: &str) {
    _ = fs::remove_dir_all(path);

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    assert!(
        Command::new(git_bin)
            .arg("init")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(b"Primera linea\nSeparador\nTercera linea\n")
        .unwrap();

    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("testfile")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("InitialCommit")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );
}
