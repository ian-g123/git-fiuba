

mod common {
    pub mod aux;
}

use std::{
    fs::{self, File},
    io::{Write},
    process::Command,
};



#[test]
fn test_ckeckout() {
    let path_repo = "./tests/data/commands/checkout_tag/repo1";
    let git_bin = "../../../../../../target/debug/git";

    fs::create_dir_all(path_repo).unwrap();

    Command::new(git_bin)
        .args(["init"])
        .current_dir(path_repo)
        .output()
        .expect("failed to initialize git repository");

    let mut file = File::create(format!("{}/testfile1", path_repo)).unwrap();
    file.write_all(b"").unwrap();

    Command::new(git_bin)
        .arg("add")
        .arg("testfile1")
        .current_dir(path_repo)
        .output()
        .expect("failed to initialize git repository");

    Command::new(git_bin)
        .arg("commit")
        .arg("-m")
        .arg("Initialcommit")
        .current_dir(path_repo)
        .output()
        .expect("failed to commit changes");

    Command::new(git_bin)
        .arg("tag")
        .arg("my_tag")
        .current_dir(path_repo)
        .output()
        .expect("failed to add tag");

    let result = Command::new(git_bin)
        .arg("checkout")
        .arg("my_tag")
        .current_dir(path_repo)
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8_lossy(&result.stderr),
        "Feature not implemented: checkout to ditached HEAD state\n"
    );

    fs::remove_dir_all(path_repo.to_string()).unwrap();
}
