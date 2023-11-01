use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

pub fn create_test_scene_1(path: &str) {
    create_base_scene(path);

    let Ok(_) = fs::copy(
        "tests/data/commands/add/testfile.txt",
        &(path.to_owned() + "/testfile.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };

    println!("Repo creado");

    assert!(Path::new(&(path.to_owned() + "/testfile.txt")).exists())
}

pub fn create_test_scene_3(path: &str) {
    create_base_scene(path);
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };

    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"file 1!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile2.txt").unwrap();
    file.write_all(b"file 2!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    assert!(Path::new(&(path.to_owned() + "/dir/testfile1.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile2.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile3.txt")).exists());
}

pub fn create_test_scene_2(path: &str) {
    create_base_scene(path);
    // copy tests/data/commands/add/dir/ contents to path.to_owned() + "/dir/"
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };
    let Ok(_) = fs::copy(
        "tests/data/commands/add/dir/testfile1.txt",
        &(path.to_owned() + "/dir/testfile1.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };
    let Ok(_) = fs::copy(
        "tests/data/commands/add/dir/testfile2.txt",
        &(path.to_owned() + "/dir/testfile2.txt"),
    ) else {
        panic!("No se pudo copiar el archivo")
    };

    assert!(Path::new(&(path.to_owned() + "/dir/testfile1.txt")).exists());
    assert!(Path::new(&(path.to_owned() + "/dir/testfile2.txt")).exists())
}

pub fn change_dir_testfile1_content(path: &str) {
    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"Cambio!").unwrap();
}

pub fn change_dir_testfile1_content_and_remove_dir_testfile2(path: &str) {
    change_dir_testfile1_content(path);

    _ = fs::remove_file(path.to_string() + "/dir/testfile2.txt").unwrap();
}

pub fn create_base_scene(path: &str) {
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

pub fn create_test_scene_4(path: &str) {
    create_test_scene_2(path);

    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile4.txt").unwrap();
    file.write_all(b"file 4!").unwrap();

    let mut file = File::create(path.to_owned() + "/testfile.txt").unwrap();
    file.write_all(b"testfile!").unwrap();
}

pub fn change_test_scene_4(path: &str) {
    let mut file = File::create(path.to_owned() + "/testfile.txt").unwrap();
    file.write_all(b"Cambio!").unwrap();
    _ = fs::remove_file(path.to_string() + "/dir/testfile3.txt").unwrap();
}

pub fn change_test_scene_4_part_2(path: &str) {
    let mut file = File::create(path.to_owned() + "/testfile.txt").unwrap();
    file.write_all(b"Cambio2!").unwrap();
    _ = fs::remove_file(path.to_string() + "/dir/testfile2.txt").unwrap();
}

pub fn create_test_scene_5(path: &str) {
    create_test_scene_2(path);
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/dir1") else {
        panic!("No se pudo crear el directorio")
    };
    let mut file = File::create(path.to_owned() + "/dir/testfile3.txt").unwrap();
    file.write_all(b"file 3!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/testfile4.txt").unwrap();
    file.write_all(b"file 4!").unwrap();

    let mut file = File::create(path.to_owned() + "/testfile.txt").unwrap();
    file.write_all(b"testfile!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/dir1/testfile5.txt").unwrap();
    file.write_all(b"file 5!").unwrap();

    let mut file = File::create(path.to_owned() + "/dir/dir1/testfile6.txt").unwrap();
    file.write_all(b"file 6!").unwrap();
}

pub fn create_test_scene_6(path: &str) {
    create_test_scene_1(path);
}
