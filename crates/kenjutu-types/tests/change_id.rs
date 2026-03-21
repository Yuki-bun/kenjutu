use std::{path::Path, process::Command};

use kenjutu_types::{ChangeId, CommitChangeIdExt};
use test_repo::TestRepo;

fn jj_change_id(dir: &Path, sha: &str) -> ChangeId {
    let output = Command::new("jj")
        .args(["log", "--no-graph", "-r", sha, "-T", "change_id"])
        .current_dir(dir)
        .output()
        .expect("failed to run jj");
    assert!(
        output.status.success(),
        "jj failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let s = String::from_utf8(output.stdout).unwrap();
    s.parse().unwrap()
}

#[test]
fn test_change_id_created_by_jj() {
    let repo = TestRepo::new().unwrap();
    let a = repo.commit("a").unwrap().created;
    let a_commit = repo.repo.find_commit(a.oid()).unwrap();
    assert_eq!(a_commit.change_id(), a.change_id)
}

#[test]
fn test_fallback_change_id_matches_jj_generates() {
    let repo = TestRepo::new().unwrap();
    let sha = repo.git_commit("git commit").unwrap().oid();
    let commit = repo.repo.find_commit(sha).unwrap();

    let from_jj = jj_change_id(repo.path(), &sha.to_string());
    let ours = commit.change_id();
    assert_eq!(from_jj, ours);
}
