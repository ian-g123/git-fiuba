use std::{env, fs, path::Path, process::Command};

#[test]
fn test_test() {
    let path = "./tests/data/commands/hash_object/repo1";
    emtpy_directory(path);
    assert!(Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .is_ok());

    let result = Command::new("../../../../../target/debug/git")
        .arg("hash-object")
        .arg("../testfile.txt")
        .arg("-w")
        .current_dir(path)
        .output();
    match result {
        Ok(output) => {
            assert_eq!(output.status.success(), true);
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                "30d74d258442c7c65512eafab474568dd706c430\n"
            );
        }
        Err(e) => {
            panic!("Error: {}", e);
        }
    }
    //check that the file was created
    assert!(Path::new(
        &(path.to_owned() + "/.git/objects/30/d74d258442c7c65512eafab474568dd706c430")
    )
    .exists());

    emtpy_directory(path);
}

fn emtpy_directory(path: &str) {
    let paths = fs::read_dir(path).unwrap();
    for path in paths {
        fs::remove_dir_all(path.unwrap().path()).unwrap();
    }
}
