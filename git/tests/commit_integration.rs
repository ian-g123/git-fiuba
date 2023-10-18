use core::panic;
use std::{fmt::format, fs, path::Path, process::Command};

use git::commands::stagin_area::StagingArea;

#[test]
fn test_single_file() {
    let path = "./tests/data/commands/commit/repo1";
    create_test_scene_1(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("cfc7f886843a5f33a324dabdb66e5fa174bd0bae")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 30d74d258442c7c65512eafab474568dd706c430    testfile.txt\n"
    );

    let HEAD = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = HEAD.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let ref_path = "./tests/data/commands/commit/repo1/.git/refs/heads/master";
    let commit_hash = fs::read_to_string(ref_path).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    let output = String::from_utf8(result.stdout).unwrap();
    let output_lines: Vec<&str> = output.split('\n').collect();
    assert_eq!(
        output_lines[0],
        "tree cfc7f886843a5f33a324dabdb66e5fa174bd0bae"
    );
    assert!(output_lines[1]
        .to_string()
        .starts_with("author Foo Bar <example@email.org>"));
    assert!(output_lines[1].to_string().ends_with(" -0300"));
    assert!(output_lines[2]
        .to_string()
        .starts_with("committer Foo Bar <example@email.org>"));
    assert!(output_lines[2].to_string().ends_with("-0300"));
    assert_eq!(output_lines[3], "");
    assert_eq!(output_lines[4], "message");

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
