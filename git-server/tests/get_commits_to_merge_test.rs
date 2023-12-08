use git_lib::git_repository::GitRepository;
use git_server::http_server_components::pull_request_components::git_repository_extension::GitRepositoryExtension;

#[test]
#[ignore = "Needs git-server/tests/data/get_commits_to_merge_test.zip unziped"]
fn test1() {
    let path = "./tests/data/get_commits_to_merge_test/test1";
    let mut sink = std::io::sink();
    let mut repo = GitRepository::open(path, &mut sink).unwrap();
    let commits_to_merge = repo
        .get_commits_to_merge("master".to_string(), "rama".to_string())
        .unwrap();
    assert_eq!(commits_to_merge.len(), 1);
    assert_eq!(commits_to_merge[0].get_message(), "ModificaciónMaster");
    let commits_to_merge = repo
        .get_commits_to_merge("rama".to_string(), "master".to_string())
        .unwrap();
    assert_eq!(commits_to_merge.len(), 1);
    assert_eq!(commits_to_merge[0].get_message(), "ModificaciónRama");
}

#[test]
#[ignore = "Needs git-server/tests/data/get_commits_to_merge_test.zip unziped"]
fn test2() {
    let path = "./tests/data/get_commits_to_merge_test/test2";
    let mut sink = std::io::sink();
    let mut repo = GitRepository::open(path, &mut sink).unwrap();
    let commits_to_merge = repo
        .get_commits_to_merge("rama2".to_string(), "master".to_string())
        .unwrap();
    assert_eq!(commits_to_merge.len(), 3);
    assert_eq!(
        commits_to_merge[0].get_message(),
        "Merge branch 'rama1' into rama2"
    );
    assert_eq!(commits_to_merge[1].get_message(), "CommitRama2");
    assert_eq!(commits_to_merge[2].get_message(), "CommitRama1");
}

#[test]
#[ignore = "Needs git-server/tests/data/get_commits_to_merge_test.zip unziped"]
fn test3() {
    let path = "./tests/data/get_commits_to_merge_test/test3";
    let mut sink = std::io::sink();
    let mut repo = GitRepository::open(path, &mut sink).unwrap();
    let commits_to_merge = repo
        .get_commits_to_merge("rama".to_string(), "master".to_string())
        .unwrap();
    assert_eq!(commits_to_merge.len(), 2);
    assert_eq!(
        commits_to_merge[0].get_message(),
        "Merge branch 'master' into rama"
    );
    assert_eq!(commits_to_merge[1].get_message(), "ModificaciónRama");
    let commits_to_merge = repo
        .get_commits_to_merge("master".to_string(), "rama".to_string())
        .unwrap();
    assert_eq!(commits_to_merge.len(), 0);
}
