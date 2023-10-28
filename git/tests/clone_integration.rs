use core::panic;
use std::{
    fs,
    path::Path,
    process::{Child, Command},
};

#[test]
#[ignore]
fn test_clone() {
    let path = "./tests/data/commands/clone/test1";

    _ = fs::remove_dir_all(format!("{}/server-repo", path));
    // let mut handle = create_base_scene_and_start_server(path.clone());
    // let id = handle.id();

    let result = Command::new("../../../../../target/debug/git")
        .arg("clone")
        .arg("git://127.1.0.0:9418/server-repo")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    // match handle.kill() {
    //     Ok(_) => {}
    //     Err(error) => {
    //         panic!("No se pudo matar el proceso {}: {}", id, error);
    //     }
    // }
    // handle.wait().unwrap();
    panic!("Pausa");

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_base_scene_and_start_server(path: &str) /*-> Child*/
{
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
        "No se pudo crear el directorio server-repo"
    );

    assert!(
        Command::new("mkdir")
            .arg("server-repo")
            .current_dir(path.to_owned() + "/server-files")
            .status()
            .is_ok(),
        "No se pudo crear el directorio server-repo"
    );

    assert!(
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path.to_owned() + "/server-files/server-repo")
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );

    assert!(
        Command::new("touch")
            .arg("testfile")
            .current_dir(path.to_owned() + "/server-files/server-repo")
            .status()
            .is_ok(),
        "No se pudo crear el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/server-files/server-repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi")
            .current_dir(path.to_owned() + "/server-files/server-repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new("touch")
            .arg("git-daemon-export-ok")
            .current_dir(path.to_owned() + "/server-files/server-repo/.git")
            .status()
            .is_ok(),
        "No se pudo crear el archivo testfile"
    );

    // Run this but instead of waiting for it to finish, run it in the background and save output to a file so that we can read it later
    // assert!(
    //     Command::new("git")
    //         .arg("daemon")
    //         .arg("--verbose")
    //         .arg("--reuseaddr")
    //         .arg("--enable=receive-pack")
    //         .arg("--base-path=.")
    //         .current_dir(path.to_owned() + "/server-repo")
    //         .status()
    //         .is_ok(),
    //     "No se pudo iniciar el daemon"
    // );

    // let handle = Command::new("git")
    //     .arg("daemon")
    //     .arg("--verbose")
    //     .arg("--reuseaddr")
    //     .arg("--enable=receive-pack")
    //     .arg("--base-path=.")
    //     .current_dir(path.to_owned() + "/server-repo")
    //     .spawn()
    //     .expect("No se pudo iniciar el daemon");

    // handle
}
