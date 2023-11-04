use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};

#[test]
fn test_push() {
    let path = "./tests/data/commands/push/test1";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone());

    let result = Command::new(git_bin)
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}\n\n", String::from_utf8(result.stdout).unwrap());

    let mut file = File::create(path.to_owned() + "/repo/testfile2").unwrap();
    file.write_all(b"contenido2\n").unwrap();

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("add")
            .arg("testfile2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    let result = Command::new("../".to_owned() + git_bin)
        .arg("push")
        .current_dir(&format!("{}/repo", path))
        .output()
        .unwrap();

    println!(
        "push vacío:\nSTDOUT:\n{}\n========\nERRORES\n========\n{}\n========",
        String::from_utf8(result.stdout).unwrap(),
        String::from_utf8(result.stderr).unwrap()
    );

    let mut file = File::create(path.to_owned() + "/repo/testfile2").unwrap();
    file.write_all(b"contenido2\n").unwrap();

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("add")
            .arg("testfile2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    let result = Command::new("../".to_owned() + git_bin)
        .arg("push")
        .current_dir(&format!("{}/repo", path))
        .output()
        .unwrap();

    println!(
        "push:\nSTDOUT:\n{}\n========\nERRORES\n========\n{}\n========",
        String::from_utf8(result.stdout).unwrap(),
        String::from_utf8(result.stderr).unwrap()
    );
    panic!();
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
            .arg("--initial-branch=master")
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
