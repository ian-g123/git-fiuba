use std::{
    fs::{self, File},
    io::{Read, Write},
    process::Command,
};

#[test]
#[ignore = "Needs server"]
fn test_push() {
    let path = "./tests/data/test1";

    let rebuild_server_repo = false;
    create_base_scene(path.clone(), rebuild_server_repo);
    if rebuild_server_repo {
        panic!();
    }
    let result = Command::new("git")
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .arg("repo")
        .current_dir(path)
        .output()
        .unwrap();
    println!(
        "===\nstderr: {}\n===",
        String::from_utf8(result.stderr).unwrap()
    );
    println!(
        "===\nstdout: {}\n===",
        String::from_utf8(result.stdout).unwrap()
    );

    let mut readme = File::open(path.to_owned() + "/repo/README.md").unwrap();
    let mut contents = String::new();
    readme.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Commit inicial\n");

    let mut file = File::create(path.to_owned() + "/repo/testfile").unwrap();
    file.write_all(b"contenido\n").unwrap();

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new("git")
            .arg("push")
            .current_dir(&format!("{}/repo", path))
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    panic!("STOP");
    let mut file = File::create(path.to_owned() + "/repo/testfile").unwrap();
    file.write_all(b"contenido\n").unwrap();

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new("../".to_owned() + "git")
            .arg("push")
            .current_dir(&format!("{}/repo", path))
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("../".to_owned() + "git")
            .arg("clone")
            .arg("git://127.1.0.0:9418/repo")
            .current_dir(path.to_owned() + "/other_user")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let mut testfile2 = File::open(path.to_owned() + "/other_user/repo/testfile2").unwrap();
    let mut contents = String::new();
    testfile2.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "contenido2\n");

    let mut readme = File::open(path.to_owned() + "/other_user/repo/README.md").unwrap();
    let mut contents = String::new();
    readme.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Commit inicial\n");

    panic!("STOP");
    _ = fs::remove_dir_all(path);
}

fn create_base_scene(path: &str, rebuild_server_repo: bool) {
    if rebuild_server_repo {
        _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
        _ = fs::remove_dir_all(format!("{}/repocan", path));
    }
    _ = fs::remove_dir_all(format!("{}/repo", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let Ok(_) = fs::create_dir_all(format!("{}/server-files/repo", path)) else {
        panic!("No se pudo crear el directorio")
    };

    let Ok(_) = fs::create_dir_all(format!("{}/other_user", path)) else {
        panic!("No se pudo crear el directorio")
    };

    assert!(
        Command::new("git")
            .arg("init")
            .arg("--initial-branch=master")
            .arg("-q")
            .arg("--bare")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );

    assert!(
        Command::new("touch")
            .arg("git-daemon-export-ok")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo crear el archivo git-daemon-export-ok"
    );

    assert!(
        Command::new("git")
            .arg("config")
            .arg("receive.denyCurrentBranch")
            .arg("ignore")
            .current_dir(path.to_owned() + "/server-files/repo")
            .status()
            .is_ok(),
        "No se pudo crear el archivo git-daemon-export-ok"
    );

    if rebuild_server_repo {
        assert!(
            Command::new("git")
                .arg("clone")
                .arg("git://127.1.0.0:9418/repo")
                .arg("repocan")
                .current_dir(path.to_owned())
                .status()
                .is_ok(),
            "No se pudo clonar con canónico"
        );
        let mut file = File::create(path.to_owned() + "/repocan/README.md").unwrap();
        file.write_all(b"Commit inicial\n").unwrap();

        assert!(
            Command::new("git")
                .arg("add")
                .arg("README.md")
                .current_dir(path.to_owned() + "/repocan")
                .status()
                .is_ok(),
            "No se pudo agregar el archivo testfile"
        );

        assert!(
            Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg("InitialCommit")
                .current_dir(path.to_owned() + "/repocan")
                .status()
                .is_ok(),
            "No se pudo hacer commit"
        );

        assert!(
            Command::new("git")
                .arg("push")
                .current_dir(path.to_owned() + "/repocan")
                .status()
                .is_ok(),
            "No se pudo hacer push"
        );
    }
}
