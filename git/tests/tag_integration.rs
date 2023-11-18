use std::{
    fs::{self, File},
    io::{Error, Read, Write},
    process::Command,
};

use common::aux::create_base_scene;

use crate::common::aux::{
    change_dir_testfile1_content_and_remove_dir_testfile2, change_test_scene_4,
    change_testfile_content, create_test_scene_2, create_test_scene_4, create_test_scene_5,
};

mod common {
    pub mod aux;
}

#[test]
fn test_create_tag() {
    let path = "./tests/data/commands/tag/repo1";

    _ = std::fs::remove_dir_all(format!("{}", path));
}
