use std::{fs, path::Path, process::Command};

use git::commands::stagin_area::StagingArea;

#[test]
fn test_single_file() {
    let path = "./tests/data/commands/add/repo1";
    create_test_scene_1(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    match std::fs::File::open(format!("{}/.git/index", path)) {
        Err(error) => panic!("No se pudo abrir el archivo: {:?}", error),
        Ok(mut file) => match StagingArea::read_from(&mut file) {
            Ok(stagin_area) => assert_eq!(
                stagin_area.files.get("testfile.txt").unwrap(),
                "30d74d258442c7c65512eafab474568dd706c430"
            ),
            Err(error) => panic!("No se pudo leer el staging area: {:?}", error),
        },
    }

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_single_file_in_root() {
    let path = "./tests/data/commands/add/repo2";
    create_test_scene_1(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    match std::fs::File::open(format!("{}/.git/index", path)) {
        Err(error) => panic!("No se pudo abrir el archivo: {:?}", error),
        Ok(mut file) => match StagingArea::read_from(&mut file) {
            Ok(stagin_area) => assert_eq!(
                stagin_area.files.get("testfile.txt").unwrap(),
                "30d74d258442c7c65512eafab474568dd706c430"
            ),
            Err(error) => panic!("No se pudo leer el staging area: {:?}", error),
        },
    }

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_two_files_in_dir() {
    let path = "./tests/data/commands/add/repo3";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    match std::fs::File::open(format!("{}/.git/index", path)) {
        Err(error) => panic!("No se pudo abrir el archivo: {:?}", error),
        Ok(mut file) => match StagingArea::read_from(&mut file) {
            Ok(stagin_area) => {
                assert_eq!(
                    stagin_area.files.get("dir/testfile1.txt").unwrap(),
                    "30d74d258442c7c65512eafab474568dd706c430"
                );
                assert_eq!(
                    stagin_area.files.get("dir/testfile2.txt").unwrap(),
                    "30d74d258442c7c65512eafab474568dd706c430"
                )
            }
            Err(error) => panic!("No se pudo leer el staging area: {:?}", error),
        },
    }

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_test_scene_1(path: &str) {
    create_base_scene(path);

    let Ok(_) = fs::copy(
        "tests/data/commands/add/testfile.txt",
        &(path.to_owned() + "/testfile.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };

    assert!(Path::new(&(path.to_owned() + "/testfile.txt")).exists())
}

fn create_test_scene_2(path: &str) {
    create_base_scene(path);
    // copy tests/data/commands/add/dir/ contents to path.to_owned() + "/dir/"
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };
    let Ok(_) = fs::copy(
        "tests/data/commands/add/dir/testfile1.txt",
        &(path.to_owned() + "/dir/testfile1.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };
    let Ok(_) = fs::copy(
        "tests/data/commands/add/dir/testfile2.txt",
        &(path.to_owned() + "/dir/testfile2.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };

    assert!(Path::new(&(path.to_owned() + "/dir/testfile1.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile2.txt")).exists())
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}", path));
    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    assert!(
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo inicializar el repositorio"
    );
}
