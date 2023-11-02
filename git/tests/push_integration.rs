use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};

#[test]
#[ignore]
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

    // println!("CLONEEE");
    // let result1 = Command::new("../".to_owned() + git_bin)
    //     .arg("push")
    //     .current_dir(&format!("{}/repo", path))
    //     .output()
    //     .unwrap();

    // println!("{}", String::from_utf8(result1.stderr).unwrap());
    // println!("{}\n\n", String::from_utf8(result1.stdout).unwrap());

    // let result = Command::new("../".to_owned() + git_bin)
    //     .arg("merge")
    //     .current_dir(&format!("{}/repo/", path))
    //     .output()
    //     .unwrap();

    println!("lol");
    // panic!();

    let mut file = File::create(path.to_owned() + "/repo/testfile").unwrap();
    file.write_all(b"contenido2\n").unwrap();
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

    // assert!(
    //     Command::new("git")
    //         .arg("checkout")
    //         .arg("-b")
    //         .arg("nueva_rama")
    //         .current_dir(path.to_owned() + "/server-files/repo")
    //         .status()
    //         .is_ok(),
    //     "No se pudo crear la nueva rama"
    // );

    // let mut file = File::create(path.to_owned() + "/server-files/repo/testfile2").unwrap();
    // file.write_all(b"contenido2\n").unwrap();

    // assert!(
    //     Command::new("git")
    //         .arg("add")
    //         .arg("testfile2")
    //         .current_dir(path.to_owned() + "/server-files/repo")
    //         .status()
    //         .is_ok(),
    //     "No se pudo agregar el archivo testfile"
    // );

    // assert!(
    //     Command::new("git")
    //         .arg("commit")
    //         .arg("-m")
    //         .arg("hi2")
    //         .current_dir(path.to_owned() + "/server-files/repo")
    //         .status()
    //         .is_ok(),
    //     "No se pudo hacer commit"
    // );
}
