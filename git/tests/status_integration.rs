use std::process::Command;

use common::aux::create_base_scene;

use crate::common::aux::{
    change_test_scene_2, change_test_scene_3, create_test_scene_1, create_test_scene_2,
    create_test_scene_3,
};

mod common {
    pub mod aux;
}

#[test]
fn test_working_tree_clean_long_format() {
    let path = "./tests/data/commands/status/repo1";

    create_base_scene(path);

    let expected = "On branch master\nnothing to commit, working tree clean\n";

    let result = Command::new("../../../../../target/debug/git")
        .arg("status")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
}

#[test]
fn test_working_tree_clean_short_format() {
    let path = "./tests/data/commands/status/repo1";

    create_base_scene(path);

    let expected = "";

    let result = Command::new("../../../../../target/debug/git")
        .arg("status")
        .arg("-s")
        .current_dir(path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
}
