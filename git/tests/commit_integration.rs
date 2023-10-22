use core::panic;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

/// Prueba que se pueda commitear un solo archivo.
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

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
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

/// Prueba que se puedan commitear únicamente los cambios agregados al staging area.
#[test]
fn test_commit_some_changes() {
    let path = "./tests/data/commands/commit/repo2";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
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

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash)
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("83b548b859cae48930179ce69adc245dda1eaa76")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree 506319ddc1dba9b08d19c136f6a3bda17e0c3726    dir\n"
    );

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("506319ddc1dba9b08d19c136f6a3bda17e0c3726")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 30d74d258442c7c65512eafab474568dd706c430    testfile1.txt\n"
    );
    let output_lines: Vec<&str> = output.split('\n').collect();
    assert_eq!(
        output_lines[0],
        "tree 83b548b859cae48930179ce69adc245dda1eaa76"
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

/// Prueba el correcto funcionamiento del flag 'all'.
#[test]
fn test_flag_all() {
    let path = "./tests/data/commands/commit/repo4";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
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

    change_test_scene_2(path);

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(testfile1_hash.trim())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "Cambio!\n");

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("e1cdfb660628b4b3ae42555b31adc0dceb076118")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree e7d329683961ce0568a1f64e112158effd9a4a04    dir\n"
    );

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("e7d329683961ce0568a1f64e112158effd9a4a04")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 9d1bdbbe7e41c96f5eb2231cc98240845610f183    testfile1.txt\n"
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba el correcto funcionamiento del flag 'all' cuando hay archivos eliminados en el
/// working tree.
#[test]
fn test_flag_all_with_deleted_files() {
    let path = "./tests/data/commands/commit/repo3";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
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

    change_test_scene_3(path);

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(testfile1_hash.trim())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "Cambio!\n");

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("e1cdfb660628b4b3ae42555b31adc0dceb076118")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree e7d329683961ce0568a1f64e112158effd9a4a04    dir\n"
    );

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("e7d329683961ce0568a1f64e112158effd9a4a04")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "100644 blob 9d1bdbbe7e41c96f5eb2231cc98240845610f183    testfile1.txt\n"
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba el correcto funcionamiento del flag 'C'.
#[test]
fn test_reuse_message() {
    let path = "./tests/data/commands/commit/repo6";
    create_test_scene_2(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let hash1 = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();

    let output_lines: Vec<&str> = output.split('\n').collect();
    let tree_hash = output_lines[0];
    let author = output_lines[1];
    let commiter = output_lines[2];
    let message = output_lines[4];

    change_test_scene_2(path);

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let hash2 = String::from_utf8(result.stdout).unwrap();

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-C")
        .arg(commit_hash)
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
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

    assert_ne!(tree_hash, output_lines[0]);
    assert_eq!(author, output_lines[2]);
    assert_eq!(commiter, output_lines[3]);
    assert_eq!(message, output_lines[5]);

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba que se puedan agregar al staging area los archivos pasados al comando Commit.
#[test]
fn test_commit_paths() {
    let path = "./tests/data/commands/commit/repo7";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
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
        .arg("hash-object")
        .arg("dir/testfile2.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile2_hash = String::from_utf8(result.stdout).unwrap();

    change_test_scene_3(path);

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let testfile1_hash = String::from_utf8(result.stdout).unwrap();

    let head = fs::read_to_string(path.to_owned() + "/.git/HEAD").unwrap();
    let (_, branch_ref) = head.split_once(' ').unwrap();
    let branch_ref = branch_ref.trim();
    let ref_path = path.to_owned() + "/.git/" + branch_ref;
    let commit_hash = fs::read_to_string(ref_path).unwrap();
    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg(commit_hash.clone())
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();
    let output = String::from_utf8(result.stdout).unwrap();
    println!("Output: \n {}", output);

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("c335058175661d87505df52ccd254045417097db")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "040000 tree c8b4bef6483a95051ee8fa218ba49312d79ec415    dir\n"
    );

    let result = Command::new("../../../../../target/debug/git")
        .arg("cat-file")
        .arg("c8b4bef6483a95051ee8fa218ba49312d79ec415")
        .arg("-p")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = format!(
        "100644 blob {}    testfile1.txt\n100644 blob {}    testfile2.txt\n",
        testfile1_hash.trim(),
        testfile2_hash.trim()
    );

    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = fs::remove_dir_all(format!("{}", path));
}

/// Prueba que no se puedan agregar al staging area los archivos pasados al comando Commit
/// que no son registrados por git.
#[test]
fn test_commit_paths_fails() {
    let path = "./tests/data/commands/commit/repo8";
    create_test_scene_3(path.clone());

    let result = Command::new("../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .arg("dir/testfile2.txt")
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

    change_test_scene_3(path);

    let result = Command::new("../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    assert!(result.status.success());
    let expected = "error: pathspec 'dir/testfile3.txt' did not match any file(s) known to git\n";
    assert_eq!(String::from_utf8(result.stderr).unwrap(), expected);

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

    println!("Repo creado");

    assert!(Path::new(&(path.to_owned() + "/testfile.txt")).exists())
}

fn create_test_scene_3(path: &str) {
    create_base_scene(path);
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };

    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"file 1!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile2.txt").unwrap();
    file.write_all(b"file 2!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    assert!(Path::new(&(path.to_owned() + "/dir/testfile1.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile2.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile3.txt")).exists());
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

fn change_test_scene_2(path: &str) {
    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();

    file.write_all(b"Cambio!").unwrap();
}

fn change_test_scene_3(path: &str) {
    change_test_scene_2(path);

    _ = fs::remove_file(path.to_string() + "/dir/testfile2.txt").unwrap();
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
