use core::panic;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    process::Command,
};

// #[ignore = "Needs server"]
#[test]
fn test_push() {
    let path = "./tests/data/test1";

    create_base_scene(path.clone());

    let result = Command::new("git")
        .arg("clone")
        .arg("git://127.1.0.0:9418/repo")
        .arg("repo")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(String::from_utf8(result.stderr)
        .unwrap()
        .starts_with("Cloning into 'repo'...\nReceiving objects:"));
    let mut readme = File::open(path.to_owned() + "/repo/README.md").unwrap();
    let mut contents = String::new();
    readme.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Commit inicial\n");

    let mut file = File::create(path.to_owned() + "/repo/testfile").unwrap();
    file.write_all(b"contenido\n").unwrap();

    assert!(
        Command::new("git")
            .arg("add")
            .arg("testfile")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    let result = Command::new("git")
        .arg("push")
        .current_dir(&format!("{}/repo", path))
        .output()
        .unwrap();
    assert!(String::from_utf8(result.stderr.clone())
        .unwrap()
        .starts_with("To git://127.1.0.0:9418/repo\n   e664ed5.."));
    assert!(String::from_utf8(result.stderr.clone())
        .unwrap()
        .ends_with(" master -> master\n"));

    // _ = fs::remove_dir_all(path);
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
    // Copy repo_backup to repo
    copy_dir_all(
        format!("{}/server-files/repo_backup", path),
        format!("{}/server-files/repo", path),
    )
    .unwrap();

    _ = fs::remove_dir_all(format!("{}/repo", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let Ok(_) = fs::create_dir_all(format!("{}/server-files/repo", path)) else {
        panic!("No se pudo crear el directorio")
    };

    let Ok(_) = fs::create_dir_all(format!("{}/other_user", path)) else {
        panic!("No se pudo crear el directorio")
    };
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
