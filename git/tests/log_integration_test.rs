use std::{fs, process::Command};

mod common {
    pub mod aux;
}

#[test]
fn testx() {
    let path = "./tests/data/commands/log";

    //create_base_scene(path);

    let expected = "";

    let result = Command::new("../../../../../../target/debug/git")
        .arg("log")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();
    println!("{}", String::from_utf8(result.stderr).unwrap());
    //assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
    println!("{}", String::from_utf8(result.stdout).unwrap(),);
    //_ = std::fs::remove_dir_all(format!("{}", path));
}