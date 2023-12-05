use core::panic;
use std::{
    fs::{self, File},
    io::{Read, Write},
    process::Command,
};

#[test]
fn test_merge_tag_on_tip_no_conflicts() {
    let path = "./tests/data/commands/merge_tag/test1";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone(), git_bin.clone());

    branch_none_overlaping_tag_on_branch_tip(path, git_bin);

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("tag_1")
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
        "Primera linea modificada en rama2\nSeparador\nTercera linea modificada en master\n"
    );

    let last_commit_description = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(last_commit_description.stdout).unwrap(),
        "Merge branch 'rama2' into master\n\n".to_string()
    );

    _ = fs::remove_dir_all(path.to_string());
}

#[test]
fn test_merge_tag_behind_tip() {
    let path = "./tests/data/commands/merge_tag/test2";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone(), git_bin.clone());

    branch_none_overlaping_tag_behind_branch_tip(path, git_bin);

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("tag_1")
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
        "Primera linea modificada en rama2\nSeparador\nTercera linea modificada en master\n"
    );

    let last_commit_description = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(last_commit_description.stdout).unwrap(),
        "Merge tag 'tag_1' into master\n\n".to_string()
    );

    _ = fs::remove_dir_all(path.to_string());
}

#[test]
fn test_merge_tag_conflicts() {
    let path = "./tests/data/commands/merge_tag/test3";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone(), git_bin.clone());

    branch_overlaping_tag(path, git_bin);

    let result = Command::new(git_bin)
        .arg("merge")
        .arg("tag_1")
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
        "Primera linea modificada en rama2\nSeparador\n<<<<<<< master\nTercera linea modificada en master\n=======\nTercera linea modificada en rama2\n>>>>>>> rama2\n"
    );

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

    let last_commit_description = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(last_commit_description.stdout).unwrap(),
        "Merge branch 'rama2' into master\n\n".to_string()
    );

    _ = fs::remove_dir_all(path.to_string());
}

fn branch_none_overlaping_tag_on_branch_tip(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("-b")
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
            .arg("modificacion_rama2_not_overlaping")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("tag")
            .arg("tag_1")
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
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_not_overlaping")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());
}

fn branch_none_overlaping_tag_behind_branch_tip(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("-b")
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
            .arg("modificacion_rama2_not_overlaping")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("tag")
            .arg("tag_1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(b"Primera linea modificada en rama2 after tag\nSeparador\nTercera linea\n")
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
            .arg("modificacion_rama2_not_overlaping_after_tag")
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
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_not_overlaping")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());
}

fn branch_overlaping_tag(path: &str, git_bin: &str) {
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("-b")
            .arg("rama2")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la rama"
    );

    let mut file = File::create(path.to_owned() + "/testfile").unwrap();
    file.write_all(
        b"Primera linea modificada en rama2\nSeparador\nTercera linea modificada en rama2\n",
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
            .arg("modificacion_rama2_overlaping")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new(git_bin)
            .arg("tag")
            .arg("tag_1")
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
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let result = Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("modificacion_local_overlaping")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", String::from_utf8(result.stdout).unwrap());
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
