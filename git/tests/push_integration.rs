use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    process::Command,
};

// Comando para iniciar daemon
// cd ../server-files/; clear; git daemon --verbose --reuseaddr --enable=receive-pack --informative-errors --base-path=. .
#[test]
#[ignore = "Needs server"]
fn test_push() {
    let path = "./tests/data/commands/push/test1";
    let git_bin = "../../../../../../target/debug/git";

    create_base_scene(path.clone());
    // panic!();
    assert!(
        Command::new(git_bin)
            .arg("clone")
            .arg("git://127.1.0.0:9418/repo")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let mut file = File::create(path.to_owned() + "/repo/testfile2").unwrap();
    file.write_all(b"contenido2\n").unwrap();

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("add")
            .arg("testfile2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("../".to_string() + git_bin)
            .arg("commit")
            .arg("-m")
            .arg("hi2")
            .current_dir(path.to_owned() + "/repo")
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    assert!(
        Command::new("../".to_owned() + git_bin)
            .arg("push")
            .current_dir(&format!("{}/repo", path))
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    assert!(
        Command::new("../".to_owned() + git_bin)
            .arg("clone")
            .arg("git://127.1.0.0:9418/repo")
            .current_dir(path.to_owned() + "/other_user")
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    let mut testfile2 = File::open(path.to_owned() + "/other_user/repo/testfile2").unwrap();
    let mut contents = String::new();
    testfile2.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "contenido2\n");

    let mut readme = File::open(path.to_owned() + "/other_user/repo/README.md").unwrap();
    let mut contents = String::new();
    readme.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Commit inicial\n");

    _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
    _ = fs::remove_dir_all(format!("{}/server-files/repo_backup_push", path));
    _ = fs::remove_dir_all(format!("{}/repo", path));
    _ = fs::remove_dir_all(format!("{}/other_user", path));
}

fn create_base_scene(path: &str) {
    _ = fs::remove_dir_all(format!("{}/repo", path));
    _ = fs::remove_dir_all(format!("{}/other_user", path));

    let Ok(_) = fs::create_dir_all(path.clone()) else {
        panic!("No se pudo crear el directorio")
    };
    let Ok(_) = fs::create_dir_all(format!("{}/server-files/repo", path)) else {
        panic!("No se pudo crear el directorio")
    };
    let Ok(_) = fs::create_dir_all(format!("{}/other_user", path)) else {
        panic!("No se pudo crear el directorio")
    };
    _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
    // Copy repo_backup to repo
    copy_dir_all(
        format!("{}/server-files/repo_backup_push", path),
        format!("{}/server-files/repo", path),
    )
    .unwrap();
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
