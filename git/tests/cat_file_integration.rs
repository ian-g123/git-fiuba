use std::{fs, path::Path, process::Command};

#[test]
fn test_cat_file_type() {
    let path = "./tests/data/commands/cat_file/repo1";
    create_test_scene(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-t")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "blob\n");

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_cat_file_size() {
    let path = "./tests/data/commands/cat_file/repo2";
    create_test_scene(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "4\n");

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_cat_file_pretty() {
    let path = "./tests/data/commands/cat_file/repo3";
    create_test_scene(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_test_scene(path: &str) {
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

    assert!(
        Command::new("../../../../../../target/debug/git")
            .arg("hash-object")
            .arg("../testfile.txt")
            .arg("-w")
            .current_dir(path)
            .output()
            .is_ok(),
        "No se pudo ejecutar el comando hash-object -w ../testfile.txt"
    );

    assert!(Path::new(
        &(path.to_owned() + "/.git/objects/30/d74d258442c7c65512eafab474568dd706c430")
    )
    .exists())
}
