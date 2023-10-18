use std::{
    env, fs,
    path::Path,
    process::{Command, Output},
};

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

fn inicialize_current_directory(path: &str) -> String {
    let Ok(current_dir) = env::current_dir() else {
        panic!("No se pudo obtener el directorio actual")
    };
    let absolute_path = format!("{}/{}", current_dir.display(), path);
    _ = fs::remove_dir_all(format!("{}", absolute_path));
    let Ok(_) = fs::create_dir_all(absolute_path.clone()) else {
        panic!("No se pudo crear el directorio")
    };
    absolute_path
}

fn output_success_verification(output: Output, absolute_path: String) {
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let aux = format!("Initialized empty Git repository in {}\n", absolute_path);
        assert_eq!(stdout, aux);
    } else {
        panic!("No hubo salida")
    }
}

fn output_err_verification(output: Output, err_str: &str) {
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stderr);
        let aux = "Argumentos inválidos\n";
        assert_eq!(stdout, aux);
    } else {
        panic!("{}", err_str)
    }
}

fn finalize_current_directory(path: String) {
    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_sub_dir() {
    let path = "tests/data/commands/init/repo1";
    let absolute_path = inicialize_current_directory(path);

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("./new_dir")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    let new_path = format!("{}/new_dir", absolute_path);
    let new_path_git = &format!("{}/.git", new_path);
    if !directories_exists(new_path_git) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(&format!("{}", new_path_git)) {
        panic!("No se pudo obtener el directorio actual")
    };

    output_success_verification(output, new_path.clone());
    finalize_current_directory(new_path)
}

#[test]
fn test_init() {
    let path = "tests/data/commands/init/repo2";
    let absolute_path = inicialize_current_directory(path);

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");
    let new_path_git = &format!("{}/.git", absolute_path);
    if !directories_exists(new_path_git) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(new_path_git) {
        panic!("No se pudo obtener el directorio actual")
    };

    output_success_verification(output, absolute_path.clone());
    finalize_current_directory(absolute_path)
}

#[test]
fn test_bare() {
    let path = "tests/data/commands/init/repo3";
    let absolute_path = inicialize_current_directory(path);

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("--bare")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    if !directories_exists(&absolute_path) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(&format!("{}", &absolute_path)) {
        panic!("No se pudo obtener el directorio actual")
    };

    output_success_verification(output, absolute_path.clone());
    finalize_current_directory(absolute_path)
}

#[test]
fn test_branch() {
    let path = "tests/data/commands/init/repo4";
    let absolute_path = inicialize_current_directory(path);

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("-b")
        .arg("taller")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    let new_path_git = &format!("{}/.git", absolute_path);
    if !directories_exists(new_path_git) {
        panic!("No se pudo obtener el directorio actual")
    };
    if !files_exists(new_path_git) {
        panic!("No se pudo obtener el directorio actual")
    };

    output_success_verification(output, absolute_path.clone());
    finalize_current_directory(absolute_path)
}

#[test]
fn test_err_two_dir() {
    let path = "tests/data/commands/init/repo5";
    let absolute_path = inicialize_current_directory(path);

    let output = Command::new("../../../../../target/debug/git")
        .arg("init")
        .arg("./new_dir1")
        .arg("./new_dir2")
        .current_dir(path.clone())
        .output()
        .expect("No se pudo ejecutar el comando");

    let err_str = "Debería dar error ya que se proporcionaron 2 rutas a diferentes directorios";
    output_err_verification(output, err_str);

    let new_path_git = &format!("{}/.git", absolute_path);
    if directories_exists(new_path_git) {
        panic!("No se tuvieron que haber creado los directorios")
    };
    if files_exists(new_path_git) {
        panic!("No se tuvieron que haber creado los archivos")
    };
    finalize_current_directory(absolute_path)
}
