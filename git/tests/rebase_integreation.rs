use std::{
    fs::{self, File},
    io::Write,
    process::Command,
    thread,
    time::Duration,
};

#[test]
fn test_without_conflict() {
    let path = "./tests/data/commands/rebase/repo1";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_without_conflict(path.clone(), git_bin);

    assert!(
        Command::new(git_bin)
            .arg("rebase")
            .arg("topic")
            .arg("master")
            .arg(path)
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_with_conflict() {
    let path = "./tests/data/commands/rebase/repo2";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_with_conflict(path.clone(), git_bin);

    // assert!(
    //     Command::new(git_bin)
    //         .arg("rebase")
    //         .arg("topic")
    //         .arg("master")
    //         .arg(path)
    //         .current_dir(path)
    //         .status()
    //         .is_ok(),
    //     "No se pudo agregar el archivo testfile"
    // );
}

fn create_scene_with_conflict(path: &str, git_bin: &str) {
    _ = fs::remove_dir_all(format!("{}", path));

    // creamos el directorio
    fs::create_dir_all(format!("{}", path)).unwrap();

    // creamos el archivo fu y ponemos un contenido que lo cambiaremos en otra rama
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido").unwrap();

    assert!(
        Command::new(git_bin)
            .arg("init")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo inicializar el repo"
    );

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("fu")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("inicial")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    thread::sleep(Duration::from_secs(1));
    // creamos la rama
    assert!(
        Command::new(git_bin)
            .arg("branch")
            .arg("topic")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la topic"
    );

    // sobreescribimos el contenido de fu con contenido2
    file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido master").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("fu")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    thread::sleep(Duration::from_secs(1));
    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("master1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    // cambiamos la rama
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("topic")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a la topic"
    );

    // sobre escribimos el contenido de fu con contenido3
    file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido topic").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("fu")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    thread::sleep(Duration::from_secs(1));
    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("topic1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    // cambiamos la rama
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a la master"
    );
}

fn create_scene_without_conflict(path: &str, git_bin: &str) {
    _ = fs::remove_dir_all(format!("{}", path));

    // creamos el directorio
    fs::create_dir_all(format!("{}", path)).unwrap();

    // creamos el archivo fu y ponemos un contenido que lo cambiaremos en otra rama
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido").unwrap();

    assert!(
        Command::new(git_bin)
            .arg("init")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo inicializar el repo"
    );

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("fu")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("inicial")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    thread::sleep(Duration::from_secs(1));
    // creamos la rama
    assert!(
        Command::new(git_bin)
            .arg("branch")
            .arg("topic")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo crear la topic"
    );

    // sobreescribimos el contenido de fu con contenido2
    file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido2").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg("fu")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    thread::sleep(Duration::from_secs(1));
    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("master1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    // cambiamos la rama
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("topic")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a la topic"
    );

    // cambiamos el contenido del archivo fu
    //file.write_all(b"contenido").unwrap();

    // creamos un nuevo archivo bar
    let mut file2 = File::create(format!("{}/bar", path)).unwrap();
    file2.write_all(b"bar").unwrap();

    // agregamos los archivos
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar los archivos"
    );

    thread::sleep(Duration::from_secs(1));
    // hacemos commit
    assert!(
        Command::new(git_bin)
            .arg("commit")
            .arg("-m")
            .arg("topic1")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    // cambiamos la rama
    assert!(
        Command::new(git_bin)
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo cambiar a la master"
    );
}
