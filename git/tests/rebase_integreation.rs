use std::{
    fs::{self, File},
    io::{Read, Write},
    process::Command,
    thread,
    time::Duration,
};

#[test]
fn test_without_conflict() {
    let path = "./tests/data/commands/rebase/repo1";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_without_conflict(path, git_bin);

    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("topic")
        .arg("master")
        .arg(path)
        .current_dir(path)
        .output();

    assert!(result.is_ok(), "No se pudo agregar el archivo testfile");

    let result_str = String::from_utf8(result.unwrap().stdout).unwrap();
    assert_eq!(
        result_str,
        "Successfully rebased and updated refs/heads/master\n".to_string()
    );

    let mut file = File::open(format!("{}/fu", path)).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "contenido master");

    let mut file = File::open(format!("{}/bar", path)).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "contenido topic");

    // ejecutamos el comando log
    let result_log = Command::new(git_bin).arg("log").current_dir(path).output();
    assert!(result_log.is_ok(), "No se pudo agregar el archivo testfile");
    let result_log_str = String::from_utf8(result_log.unwrap().stdout).unwrap();
    let result_log_str_vec = get_commits_and_branches(result_log_str);

    let expected_log = format!(
        "[(\"master1\", Some(\"master\")), (\"topic1\", Some(\"topic\")), (\"inicial\", None)]"
    );
    assert_eq!(result_log_str_vec, expected_log);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_with_conflict() {
    let path = "./tests/data/commands/rebase/repo2";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_with_conflict(path, git_bin);

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

    // cambiamos el archivo fu y ponemos contenido contenido topic por el conflicto
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido topic").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    // hacemos rebase --continue
    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("--continue")
        .current_dir(path)
        .output();

    assert!(
        result.is_ok(),
        "No se realizó correctamente el rebase --continue"
    );

    let result_str = String::from_utf8(result.unwrap().stdout).unwrap();
    assert_eq!(
        result_str,
        "Successfully rebased and updated refs/heads/master\n".to_string()
    );

    // ejecutamos el comando log
    let result_log = Command::new(git_bin).arg("log").current_dir(path).output();
    assert!(result_log.is_ok(), "No se pudo agregar el archivo testfile");
    let result_log_str = String::from_utf8(result_log.unwrap().stdout).unwrap();
    let result_log_str_vec = get_commits_and_branches(result_log_str);

    let expected_log = format!(
        "[(\"master1\", Some(\"master\")), (\"topic1\", Some(\"topic\")), (\"inicial\", None)]"
    );
    assert_eq!(result_log_str_vec, expected_log);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_with_conflict_with_1_argument() {
    let path = "./tests/data/commands/rebase/repo3";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_with_conflict(path, git_bin);

    assert!(
        Command::new(git_bin)
            .arg("rebase")
            .arg("topic")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo testfile"
    );

    // cambiamos el archivo fu y ponemos contenido contenido topic por el conflicto
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido topic").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    // hacemos rebase --continue
    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("--continue")
        .current_dir(path)
        .output();

    assert!(
        result.is_ok(),
        "No se realizó correctamente el rebase --continue"
    );

    let result_str = String::from_utf8(result.unwrap().stdout).unwrap();
    assert_eq!(
        result_str,
        "Successfully rebased and updated refs/heads/master\n".to_string()
    );

    // ejecutamos el comando log
    let result_log = Command::new(git_bin).arg("log").current_dir(path).output();
    assert!(result_log.is_ok(), "No se pudo agregar el archivo testfile");
    let result_log_str = String::from_utf8(result_log.unwrap().stdout).unwrap();
    let result_log_str_vec = get_commits_and_branches(result_log_str);

    let expected_log = format!(
        "[(\"master1\", Some(\"master\")), (\"topic1\", Some(\"topic\")), (\"inicial\", None)]"
    );
    assert_eq!(result_log_str_vec, expected_log);
    _ = fs::remove_dir_all(format!("{}", path));
}

fn read_file(file_path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(file_path)
}

#[test]
fn test_with_conflict_heavy() {
    let path = "./tests/data/commands/rebase/repo4";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_with_conflict_heavy(path, git_bin);

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

    // cambiamos el archivo fu y ponemos contenido contenido topic por el conflicto
    let mut file = File::create(format!("{}/fu", path)).unwrap();
    file.write_all(b"contenido master1 fu").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );
    thread::sleep(Duration::from_secs(1));

    // hacemos rebase --continue
    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("--continue")
        .current_dir(path)
        .output();

    assert!(
        result.is_ok(),
        "No se realizó correctamente el rebase --continue"
    );

    let content = read_file(&format!("{}/fu", path)).expect("Error al leer el archivo");
    assert_eq!(content, "contenido master1 fu");

    // cambiamos el archivo fu y ponemos contenido contenido topic por el conflicto
    let mut file = File::create(format!("{}/bar", path)).unwrap();
    file.write_all(b"contenido topic2 1").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo fu"
    );

    thread::sleep(Duration::from_secs(1));

    // hacemos rebase --continue
    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("--continue")
        .current_dir(path)
        .output();

    assert!(
        result.is_ok(),
        "No se realizó correctamente el rebase --continue"
    );

    let content = read_file(&format!("{}/fu", path)).expect("Error al leer el archivo");
    assert_eq!(content, "contenido master1 fu");

    // cambiamos el archivo fu y ponemos contenido contenido topic por el conflicto
    let mut file = File::create(format!("{}/bar", path)).unwrap();
    file.write_all(b"contenido topic2 bar 2").unwrap();

    // agregamos el archivo fu
    assert!(
        Command::new(git_bin)
            .arg("add")
            .arg(".")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo agregar el archivo bar"
    );

    thread::sleep(Duration::from_secs(1));

    // hacemos rebase --continue
    let result = Command::new(git_bin)
        .arg("rebase")
        .arg("--continue")
        .current_dir(path)
        .output();

    let result_str = String::from_utf8(result.unwrap().stdout).unwrap();
    assert_eq!(
        result_str,
        "Successfully rebased and updated refs/heads/master\n".to_string()
    );

    let content = read_file(&format!("{}/fu", path)).expect("Error al leer el archivo");
    assert_eq!(content, "contenido master1 fu");

    let content = read_file(&format!("{}/bar", path)).expect("Error al leer el archivo");
    assert_eq!(content, "contenido topic2 bar 2");

    // ejecutamos el comando log
    let result_log = Command::new(git_bin).arg("log").current_dir(path).output();
    assert!(result_log.is_ok(), "No se pudo hacer log");
    let result_log_str = String::from_utf8(result_log.unwrap().stdout).unwrap();
    let result_log_str_vec = get_commits_and_branches(result_log_str);

    let expected_log = format!(
        "[(\"master3\", Some(\"master\")), (\"master2\", None), (\"master1\", None), (\"topic2\", Some(\"topic\")), (\"topic1\", None), (\"inicial\", None)]"
    );
    assert_eq!(result_log_str_vec, expected_log);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_with_conflict_abort() {
    let path = "./tests/data/commands/rebase/repo5";
    let git_bin = "../../../../../../target/debug/git";

    create_scene_with_conflict_heavy(path, git_bin);

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

    // cambiamos el contenido del archivo fu
    //file.write_all(b"contenido").unwrap();

    // creamos un nuevo archivo bar
    let mut file2 = File::create(format!("{}/bar", path)).unwrap();
    file2.write_all(b"contenido topic").unwrap();

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
    thread::sleep(Duration::from_secs(1));
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
    thread::sleep(Duration::from_secs(1));
}

fn create_scene_with_conflict_heavy(path: &str, git_bin: &str) {
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
    file.write_all(b"contenido master1 fu").unwrap();

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
    file.write_all(b"contenido topic1 fu").unwrap();

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

    // creamos un nuevo archivo bar
    let mut file2 = File::create(format!("{}/bar", path)).unwrap();
    file2.write_all(b"contenido topic2 bar").unwrap();
    // creamos un nuevo archivo bar
    let mut file3 = File::create(format!("{}/woo", path)).unwrap();
    file3.write_all(b"contenido topic2 woo").unwrap();

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
            .arg("topic2")
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

    // creamos un nuevo archivo bar
    let mut file4 = File::create(format!("{}/bar", path)).unwrap();
    file4.write_all(b"contenido master2 bar").unwrap();
    // // creamos un nuevo archivo bar
    // let mut file3 = File::create(format!("{}/woo", path)).unwrap();
    // file3.write_all(b"contenido topic woo").unwrap();

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
            .arg("master2")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );

    // creamos un nuevo archivo bar
    let mut file5 = File::create(format!("{}/bar", path)).unwrap();
    file5.write_all(b"contenido master3 bar").unwrap();
    // // creamos un nuevo archivo bar
    // let mut file3 = File::create(format!("{}/woo", path)).unwrap();
    // file3.write_all(b"contenido topic woo").unwrap();

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
            .arg("master3")
            .current_dir(path)
            .status()
            .is_ok(),
        "No se pudo hacer commit"
    );
    thread::sleep(Duration::from_secs(1));
}

fn get_commits_and_branches(output: String) -> String {
    let mut commits: Vec<(String, Option<&str>)> = Vec::new();
    let commit_parts: Vec<&str> = output.split("commit ").collect();

    for part in commit_parts.iter().skip(1) {
        let lines: Vec<&str> = part.lines().collect();
        let index_msj_line = 4;
        let message = lines[index_msj_line].trim();

        let mut branch: Option<&str> = None;
        let branch_vec = lines[0].splitn(2, " ").collect::<Vec<&str>>();
        if branch_vec.len() > 1 {
            branch = Some(branch_vec[1]);
        }

        commits.push((message.to_string(), branch));
    }

    let result = format!("{:?}", commits);
    return result;
}
