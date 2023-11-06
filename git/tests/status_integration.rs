use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    process::Command,
};

use common::aux::create_base_scene;

use crate::common::aux::{
    change_test_scene_4, create_test_scene_2, create_test_scene_4, create_test_scene_5,
};

mod common {
    pub mod aux;
}

#[test]
fn test_working_tree_clean_long_format() {
    let path = "./tests/data/commands/status/repo1";

    create_base_scene(path);

    let expected = "On branch master\n\nNo commits yet\n\nnothing to commit (create/copy files and use \"git add\" to track)\n";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();
    println!("{}", String::from_utf8(result.stderr).unwrap());
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
    _ = std::fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_working_tree_clean_short_format() {
    let path = "./tests/data/commands/status/repo2";

    create_base_scene(path);

    let expected = "";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_no_changes_added_to_commit() {
    let path = "./tests/data/commands/status/repo3";

    create_test_scene_2(path);

    let expected = "On branch master\n\nNo commits yet\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n	dir/\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\"\n";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let result = String::from_utf8(result.stdout).unwrap();
    println!("{}", result);
    assert_eq!(result, expected);
    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_no_changes_added_to_commit_short_format() {
    let path = "./tests/data/commands/status/repo4";

    create_test_scene_2(path);

    let expected = "?? dir/\n";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    let result = String::from_utf8(result.stdout).unwrap();
    println!("{}", result);
    assert_eq!(result, expected);
    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn general_test_long() {
    let path = "./tests/data/commands/status/repo5";
    create_test_scene_4(path);
    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = "On branch master\n\nNo commits yet\n\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n\tnew file:   dir/testfile3.txt\n\tnew file:   testfile.txt\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\tdir/testfile1.txt\n\tdir/testfile2.txt\n\tdir/testfile4.txt\n\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = "On branch master\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\tdir/testfile1.txt\n\tdir/testfile2.txt\n\tdir/testfile4.txt\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\"\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    change_test_scene_4(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "On branch master\nChanges not staged for commit:\n  (use \"git add/rm <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n\tdeleted:   dir/testfile3.txt\n\tmodified:   testfile.txt\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\tdir/testfile1.txt\n\tdir/testfile2.txt\n\tdir/testfile4.txt\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\"\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "On branch master\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n\tdeleted:   dir/testfile3.txt\n\nChanges not staged for commit:\n  (use \"git add/rm <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n\tmodified:   testfile.txt\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\tdir/testfile1.txt\n\tdir/testfile2.txt\n\tdir/testfile4.txt\n\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn general_test_short() {
    let path = "./tests/data/commands/status/repo6";
    create_test_scene_4(path);
    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = "?? dir/testfile1.txt\n?? dir/testfile2.txt\nA  dir/testfile3.txt\n?? dir/testfile4.txt\nA  testfile.txt\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("commit")
        .arg("-m")
        .arg("message")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    let expected = "?? dir/testfile1.txt\n?? dir/testfile2.txt\n?? dir/testfile4.txt\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    change_test_scene_4(path);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    let expected =
        "?? dir/testfile1.txt\n?? dir/testfile2.txt\n D dir/testfile3.txt\n?? dir/testfile4.txt\n M testfile.txt\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile3.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("--short")
        .current_dir(path)
        .output()
        .unwrap();
    let expected =
    "?? dir/testfile1.txt\n?? dir/testfile2.txt\nD  dir/testfile3.txt\n?? dir/testfile4.txt\n M testfile.txt\n";
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_untracked_folder() {
    let path = "./tests/data/commands/status/repo7";

    create_test_scene_5(path);

    let expected = "?? dir/\n?? testfile.txt\n";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();

    let result = String::from_utf8(result.stdout).unwrap();
    assert_eq!(result, expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "?? dir/\nA  testfile.txt\n";
    let result = String::from_utf8(result.stdout).unwrap();
    assert_eq!(result, expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "?? dir/dir1/\nA  dir/testfile1.txt\n?? dir/testfile2.txt\n?? dir/testfile3.txt\n?? dir/testfile4.txt\nA  testfile.txt\n";
    let result = String::from_utf8(result.stdout).unwrap();
    assert_eq!(result, expected);

    _ = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("dir/dir1/testfile5.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    let expected = "A  dir/dir1/testfile5.txt\n?? dir/dir1/testfile6.txt\nA  dir/testfile1.txt\n?? dir/testfile2.txt\n?? dir/testfile3.txt\n?? dir/testfile4.txt\nA  testfile.txt\n";
    let result = String::from_utf8(result.stdout).unwrap();
    assert_eq!(result, expected);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_status_remote_info() {
    let path = "./tests/data/commands/status/repo8";
    create_base_scene(path);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();
    let result = String::from_utf8(result.stderr).unwrap();
    println!("{}", result);
    //_ = fs::remove_dir_all(format!("{}", path));
}

/* #[test]
fn test_status_unmerged() {
    let path = "./tests/data/commands/status/repo9";
    create_base_scene_merge(path);
    //_ = fs::remove_dir_all(format!("{}", path));
}

fn create_base_scene_merge(path: &str) {
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

    let mut file = fs::File::create(path.to_owned() + "/server-files/repo/testfile").unwrap();
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
 */
