use std::{fs, io::Write, path::Path, process::Command};

use git_lib::staging_area_components::staging_area::StagingArea;

#[test]
fn test_single_file() {
    let path = "./tests/data/commands/rm/repo1";
    create_test_scene_1(path.clone());

    let result = Command::new("../../../../../../target/debug/git")
        .arg("add")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    let result = Command::new("../../../../../../target/debug/git")
        .arg("cat-file")
        .arg("-p")
        .arg("30d74d258442c7c65512eafab474568dd706c430")
        .current_dir(path)
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(result.stdout).unwrap(), "test\n");

    match fs::File::open(format!("{}/.git/index", path)) {
        Err(error) => panic!("No se pudo abrir el archivo: {:?}", error),
        Ok(mut file) => match StagingArea::read_from(&mut file, "") {
            Ok(stagin_area) => assert_eq!(
                stagin_area.get_files().get("testfile.txt").unwrap(),
                "30d74d258442c7c65512eafab474568dd706c430"
            ),
            Err(error) => panic!("No se pudo leer el staging area: {:?}", error),
        },
    }

    let result = Command::new("../../../../../../target/debug/git")
        .arg("rm")
        .arg("testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    println!("{}", String::from_utf8(result.stderr).unwrap());
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "");

    match fs::File::open(format!("{}/.git/index", path)) {
        Ok(_) => (),
        Err(error) => panic!("Se pudo abrir el archivo: {:?}", error),
    }

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_test_scene_1(path: &str) {
    create_base_scene(path);

    let file = fs::File::create(path.to_owned() + "/testfile.txt");
    match file {
        Ok(mut file) => {
            let _ = file.write_all(b"test");
        }
        Err(error) => panic!("No se pudo crear el archivo: {:?}", error),
    }

    assert!(Path::new(&(path.to_owned() + "/testfile.txt")).exists())
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
