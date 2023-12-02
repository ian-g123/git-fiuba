use std::{
    fs::{self, File, OpenOptions},
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
fn test_comment() {
    let path = "./tests/data/commands/check_ignore/repo1";
    create_check_ignore_scene(path);

    write_to_gitignore(path, false, "# *.txt\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!(expected, stdout);
    assert_eq!(expected, stderr);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .arg("-n")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir/testfile1.txt\n";

    assert_eq!(expected, stdout);
    assert_eq!("", stderr);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_gitignore_root_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo2";
    create_check_ignore_scene(path);

    write_to_gitignore(path, false, "# pattern\n*.txt\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".gitignore:2:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("--non-matching")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_gitignore_next_level_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo3";
    create_check_ignore_scene(path);

    write_to_gitignore(path, true, "# pattern\n*.txt\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/.gitignore:2:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_exclude_ends_with() {
    let path = "./tests/data/commands/check_ignore/repo4";
    create_check_ignore_scene(path);

    write_to_exclude(path, "# pattern\n*.txt\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "dir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".git/info/exclude:8:*.txt\tdir/testfile1.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("--non-matching")
        .arg("*")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_order_negate() {
    let path = "./tests/data/commands/check_ignore/repo5";
    create_check_ignore_scene(path);

    write_to_exclude(path, "*.txt\n", true);
    write_to_gitignore(path, false, "!a.txt\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("a.txt")
        .arg("--verbose")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".git/info/exclude:7:*.txt\tdir/testfile1.txt\n.gitignore:1:!a.txt\ta.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, " !a.txt\n", false);
    write_to_gitignore(path, false, "*.txt\n", false);
    write_to_gitignore(path, true, "*.txt\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("a.txt")
        .arg("-v")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".git/info/exclude:7:!a.txt\ta.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_ends_with_general() {
    let path = "./tests/data/commands/check_ignore/repo7";
    create_check_ignore_scene(path);
    write_to_exclude(path, "*a/name/\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("abba/name/")
        .arg("abba/name/otro")
        .arg("a/abba/name/")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "abba/name/\nabba/name/otro\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "*a/name\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("a/name")
        .arg("abba/name/")
        .arg("abba/name/otro")
        .arg("a/abba/name/")
        .arg("abc_name")
        .arg("a/name/")
        .arg("name")
        .arg("name_abc")
        .arg("name/a/b/c")
        .arg("abc_name/")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "a/name\nabba/name/\nabba/name/otro\na/name/\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "*name\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("abba/name/")
        .arg("abba/name/otro")
        .arg("a/abba/name/")
        .arg("abc_name")
        .arg("a/name/")
        .arg("name")
        .arg("name_abc")
        .arg("name/a/b/c")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected =
        "abba/name/\nabba/name/otro\na/abba/name/\nabc_name\na/name/\nname\nname/a/b/c\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "*name/\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("abba/name/")
        .arg("abba/name/otro")
        .arg("a/abba/name/")
        .arg("abc_name")
        .arg("a/name/")
        .arg("name")
        .arg("name_abc")
        .arg("name/a/b/c")
        .arg("abc_name/")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "abba/name/\nabba/name/otro\na/abba/name/\na/name/\nname/a/b/c\nabc_name/\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "*/name\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("abba/name/")
        .arg("abba/name/otro")
        .arg("a/abba/name/")
        .arg("abc_name")
        .arg("a/name/")
        .arg("name")
        .arg("name_abc")
        .arg("name/a/b/c")
        .arg("abc_name/")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "abba/name/\nabba/name/otro\na/name/\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_starts_with() {
    let path = "./tests/data/commands/check_ignore/repo8";
    create_check_ignore_scene(path);

    write_to_exclude(path, "name/*\n", true);
    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name/a.txt")
        .arg("b/name/a.txt")
        .arg("name/")
        .arg("name")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name/a.txt\nname/\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "a/name/*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("b/a/name/a.txt")
        .arg("a/name/a.txt")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "a/name/a.txt\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "name*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name/a.txt")
        .arg("b/name/a.txt")
        .arg("a/b/name/a.txt")
        .arg("name/")
        .arg("name")
        .arg("name_and_sth_more")
        .arg("name/a/b")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected =
        "name/a.txt\nb/name/a.txt\na/b/name/a.txt\nname/\nname\nname_and_sth_more\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "/name*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name/a.txt")
        .arg("b/name/a.txt")
        .arg("a/b/name/a.txt")
        .arg("name/")
        .arg("name")
        .arg("name_and_sth_more")
        .arg("name/a/b")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name/a.txt\nname/\nname\nname_and_sth_more\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "/name/*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name/a.txt")
        .arg("b/name/a.txt")
        .arg("a/b/name/a.txt")
        .arg("name/")
        .arg("name")
        .arg("name_and_sth_more")
        .arg("name/a/b")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name/a.txt\nname/\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "a/name*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name/a.txt")
        .arg("b/name/a.txt")
        .arg("a/b/name/a.txt")
        .arg("a/name")
        .arg("a/name_a")
        .arg("a/name/")
        .arg("a/name/b")
        .arg("name/")
        .arg("name")
        .arg("name_and_sth_more")
        .arg("name/a/b")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "a/name\na/name_a\na/name/\na/name/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_non_matching() {
    let path = "./tests/data/commands/check_ignore/repo6";
    create_test_scene_5(path);
    write_to_exclude(path, "dir*\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("dir/testfile1.txt")
        .arg("dir/dir1/testfile5.txt")
        .arg("sth")
        .arg("--verbose")
        .arg("-n")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = ".git/info/exclude:7:dir*\tdir/testfile1.txt\n.git/info/exclude:7:dir*\tdir/dir1/testfile5.txt\n::\tsth\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "test*\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("*")
        .arg("text")
        .arg("")
        .arg("--verbose")
        .arg("-n")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "::\tdir\n.git/info/exclude:7:test*\ttestfile.txt\n::\ttext\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

#[test]
fn test_relative_patterns() {
    let path = "./tests/data/commands/check_ignore/repo9";

    create_check_ignore_scene(path);
    write_to_exclude(path, "/name\n", true);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name")
        .arg("name_abc")
        .arg("name.txt")
        .arg("name/")
        .arg("name/a")
        .arg("name/a/b")
        .arg("abc_name")
        .arg("a/name")
        .arg("a/b/name")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name\nname/\nname/a\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "/name/\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name")
        .arg("name_abc")
        .arg("name.txt")
        .arg("name/")
        .arg("name/a")
        .arg("name/a/b")
        .arg("abc_name")
        .arg("a/name")
        .arg("a/b/name")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name/\nname/a\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    write_to_exclude(path, "/name/a\n", false);

    let result = Command::new("../../../../../../target/debug/git")
        .arg("check-ignore")
        .arg("name")
        .arg("name_abc")
        .arg("name.txt")
        .arg("name/")
        .arg("name/a")
        .arg("name/a/b")
        .arg("abc_name")
        .arg("a/name")
        .arg("a/b/name")
        .current_dir(path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    let expected = "name/a\nname/a/b\n";

    assert_eq!("", stderr);
    assert_eq!(expected, stdout);

    _ = fs::remove_dir_all(format!("{}", path));
}

fn create_check_ignore_scene(path: &str) {
    create_base_scene(path);
    let Ok(_) = fs::create_dir_all(path.to_owned() + "/dir/") else {
        panic!("No se pudo crear el directorio")
    };

    let mut file = File::create(path.to_owned() + "/dir/testfile1.txt").unwrap();
    file.write_all(b"file 1!").unwrap();

    _ = File::create(path.to_owned() + "/.gitignore").unwrap();

    _ = File::create(path.to_owned() + "/dir/.gitignore").unwrap();
}

fn write_to_gitignore(path: &str, dir: bool, content: &str, append: bool) {
    let path = if dir {
        path.to_owned() + "/dir/.gitignore"
    } else {
        path.to_owned() + "/.gitignore"
    };
    let mut file = OpenOptions::new()
        .write(true)
        .append(append)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn write_to_exclude(path: &str, content: &str, append: bool) {
    let path = path.to_owned() + "/.git/info/exclude";
    let base = "# git ls-files --others --exclude-from=.git/info/exclude\n# Lines that start with '#' are comments.\n# For a project mostly in C, the following would be a good set of\n# exclude patterns (uncomment them if you want to use them):\n# *.[oa]\n# *~\n";

    if !append {
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(base.as_bytes()).unwrap();
    }

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}
