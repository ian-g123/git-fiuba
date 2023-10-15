use std::{env, fs, path::Path, process::Command};

fn directory_exists(path: &String) -> bool {
    let path = Path::new(path);
    path.is_dir()
}

fn directories_exists(path: &String) -> bool {
    if directory_exists(&format!("{}/{}", path, &"objects".to_string()))
        && directory_exists(&format!("{}/{}", path, &"branches".to_string()))
        && directory_exists(&format!("{}/{}", path, &"objects/info".to_string()))
        && directory_exists(&format!("{}/{}", path, &"objects/pack".to_string()))
        && directory_exists(&format!("{}/{}", path, &"refs".to_string()))
        && directory_exists(&format!("{}/{}", path, &"refs/heads".to_string()))
        && directory_exists(&format!("{}/{}", path, &"refs/tags".to_string()))
    {
        return true;
    };
    return false;
}

fn file_exists(path: &String) -> bool {
    let path = Path::new(path);
    path.is_file()
}

fn files_exists(path: &String) -> bool {
    if file_exists(&format!("{}/{}", path, &"HEAD".to_string())) {
        return true;
    };
    return false;
}

#[test]
fn test_sub_dir() {
    let path = "tests/data/commands/init/repo1";
    let Ok(current_dir) = env::current_dir() else {
        panic!("No se pudo obtener el directorio actual")
    };
    let absolute_path = format!("{}/{}", current_dir.display(), path);
    _ = fs::remove_dir_all(format!("{}", absolute_path));
    let Ok(_) = fs::create_dir_all(absolute_path.clone()) else {
        panic!("No se pudo crear el directorio")
    };
    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("-q")
        .arg("./new_dir")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    if !directories_exists(&format!("{}/{}/.git", absolute_path, "new_dir".to_string())) {
        panic!("No se pudo obtener el directorio actual")
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let aux = format!(
            "Initialized empty Git repository in {}/new_dir\n",
            absolute_path
        );
        assert_eq!(stdout, aux);
    } else {
        panic!("No hubo salida")
    }
    _ = fs::remove_dir_all(format!("{}", absolute_path));
}

#[test]
fn test_init() {
    let path = "tests/data/commands/init/repo2";
    let Ok(current_dir) = env::current_dir() else {
        panic!("No se pudo obtener el directorio actual")
    };
    let absolute_path = format!("{}/{}", current_dir.display(), path);
    _ = fs::remove_dir_all(format!("{}", absolute_path));
    let Ok(_) = fs::create_dir_all(absolute_path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("-q")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    if !directories_exists(&format!("{}/.git", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(&format!("{}/.git", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let aux = format!("Initialized empty Git repository in {}\n", absolute_path);
        assert_eq!(stdout, aux);
    } else {
        panic!("No hubo salida")
    }
    _ = fs::remove_dir_all(format!("{}", absolute_path));
}

#[test]
fn test_bare() {
    let path = "tests/data/commands/init/repo3";
    let Ok(current_dir) = env::current_dir() else {
        panic!("No se pudo obtener el directorio actual")
    };
    let absolute_path = format!("{}/{}", current_dir.display(), path);
    _ = fs::remove_dir_all(format!("{}", absolute_path));
    let Ok(_) = fs::create_dir_all(absolute_path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("-q")
        .arg("--bare")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    if !directories_exists(&format!("{}", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(&format!("{}", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let aux = format!("Initialized empty Git repository in {}\n", absolute_path);
        assert_eq!(stdout, aux);
    } else {
        panic!("No hubo salida")
    }
    _ = fs::remove_dir_all(format!("{}", absolute_path));
}

#[test]
fn test_branch() {
    let path = "tests/data/commands/init/repo4";
    let Ok(current_dir) = env::current_dir() else {
        panic!("No se pudo obtener el directorio actual")
    };
    let absolute_path = format!("{}/{}", current_dir.display(), path);
    _ = fs::remove_dir_all(format!("{}", absolute_path));
    let Ok(_) = fs::create_dir_all(absolute_path.clone()) else {
        panic!("No se pudo crear el directorio")
    };

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("-q")
        .arg("-b")
        .arg("taller")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    if !directories_exists(&format!("{}/.git", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(&format!("{}/.git", absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let aux = format!("Initialized empty Git repository in {}\n", absolute_path);
        assert_eq!(stdout, aux);
    } else {
        panic!("No hubo salida")
    }
    _ = fs::remove_dir_all(format!("{}", absolute_path));
}
