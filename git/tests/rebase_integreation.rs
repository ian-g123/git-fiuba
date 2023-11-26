use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};

#[test]
fn test_push() {
    let path = "./tests/data/commands/rebase";
    let git_bin = "../../../../../target/debug/git";

    create_base_scene1(path.clone(), git_bin);

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

fn create_base_scene1(path: &str, git_bin: &str) {
    _ = fs::remove_dir_all(format!("{}", path));

    // creamos el directorio
    fs::create_dir_all(format!("{}", path)).unwrap();

    // creamos el archivo fu y ponemos un contenido que lo cambiaremos en otra rama
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"principal\n").unwrap();

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
            .arg("principal")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

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

    // cambiamos el contenido del archivo fu
    file.write_all(b"contenido master\n").unwrap();

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
            .arg("master 1")
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
    file.write_all(b"contenido topict").unwrap();

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
            .arg("topic 1")
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
