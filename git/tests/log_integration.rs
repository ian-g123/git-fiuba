use std::{fs, process::Command};

mod common {
    pub mod aux;
}

#[test]
#[ignore = "./tests/data/commands/log must be extracted before running the test"]
fn testx() {
    let path = "./tests/data/commands/log";
    let result = Command::new("../../../../../target/debug/git")
        .arg("log")
        .arg("--all")
        .current_dir(path)
        .output()
        .unwrap();
    let obtained = String::from_utf8(result.stdout).unwrap();
    println!("{}", String::from_utf8(result.stderr).unwrap());
    println!("{}", obtained);

    let expected = "commit d9227de41004b217dc80699c93816b9f35127313\nMerge: 2018bd4 3de77b6\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-30 00:03:45\n\n    Merge branch 'meli' into ian\ncommit 2018bd4d0e4cb07872f5bc3aac8c69b91db764a9\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-30 00:02:08\n\n    commit5\n\ncommit 724672fe124e0d27e5e1682e1f31b0a1de17be3b\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-30 00:01:48\n\n    commit4\n\ncommit 3de77b643006cfa227f77c57cc3eecbd140214c5\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-30 00:01:07\n\n    commit3\n\ncommit 5e0b574af421a7ada1e47c9fcad216a2c254e458\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-30 00:00:36\n\n    commit2\n\ncommit 0af2f837bc8dda145c07e8539088a85592cba872\nAuthor: melijauregui <mjauregui@fi.uba.ar>\nDate: 2023-10-29 23:59:29\n\n    first commit\n\n";
    assert_eq!(obtained, expected);
    _ = fs::remove_dir_all(path);
}
