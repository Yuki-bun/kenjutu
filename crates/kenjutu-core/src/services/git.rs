use std::env;

use git2::{AutotagOption, Commit, Cred, FetchOptions, RemoteCallbacks, Repository};

use kenjutu_types::{ChangeId, CommitId};

use crate::models::PRCommit;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
}

pub fn open_repository(local_dir: &str) -> Result<Repository> {
    Repository::open(local_dir).map_err(|_| Error::RepoNotFound(local_dir.to_string()))
}

pub fn get_or_fetch_commit(repo: &Repository, commit_id: CommitId) -> Result<Commit<'_>> {
    let oid = commit_id.oid();
    if let Ok(commit) = repo.find_commit(oid) {
        return Ok(commit);
    }

    let mut remote = repo.find_remote("origin")?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        // TODO: Support configurable SSH key paths
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);
    // Ensure we don't accidentally pull in tags we don't want
    fo.download_tags(AutotagOption::None);

    // Don't create a ref
    let refspec = format!("{}:", oid);

    remote.fetch(&[&refspec], Some(&mut fo), None)?;

    repo.find_commit(oid)
        .map_err(|_| Error::CommitNotFound(oid.to_string()))
}



pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
    commit
        .header_field_bytes("change-id")
        .ok()
        .and_then(|bytes| bytes.as_str().map(|s| s.to_string()))
        .and_then(|s| s.as_str().try_into().ok())
}

const REVERSE_HEX_CHARS: &[u8; 16] = b"zyxwvutsrqponmlk";

fn reverse_hex_encode(data: &[u8]) -> String {
    let encoded: Vec<u8> = data
        .iter()
        .flat_map(|b| {
            [
                REVERSE_HEX_CHARS[(*b >> 4) as usize],
                REVERSE_HEX_CHARS[(*b & 0x0f) as usize],
            ]
        })
        .collect();
    String::from_utf8(encoded).unwrap()
}

/// Deterministically creates a ChangeId from a git commit SHA.
///
/// Ports jj's `synthetic_change_id_from_git_commit_id` algorithm:
/// 1. Take bytes [4..20] of the 20-byte SHA-1 commit hash
/// 2. Reverse the byte order
/// 3. Reverse bits within each byte
/// 4. Encode as reverse hex (32 ASCII characters)
pub fn synthetic_change_id(commit_id: CommitId) -> ChangeId {
    let oid = commit_id.oid();
    let sha_bytes = oid.as_bytes();
    let raw: Vec<u8> = sha_bytes[4..20]
        .iter()
        .rev()
        .map(|b| b.reverse_bits())
        .collect();
    let hex_string = reverse_hex_encode(&raw);
    ChangeId::try_from(hex_string.as_str()).unwrap()
}

/// Returns the change-id from the commit header if present,
/// otherwise computes a synthetic change-id from the commit SHA.
pub fn get_change_id_or_synthetic(commit: &Commit<'_>) -> ChangeId {
    get_change_id(commit).unwrap_or_else(|| synthetic_change_id(CommitId::from(commit.id())))
}

/// Walk commits in the range `base..head` (excluding base, including head),
/// returning them in newest-first order.
pub fn get_commits_in_range(
    repo: &Repository,
    base: CommitId,
    head: CommitId,
) -> Result<Vec<PRCommit>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    revwalk.push(head.oid())?;
    revwalk.hide(base.oid())?;

    let mut commits = Vec::new();
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let change_id = get_change_id_or_synthetic(&commit);

        let message = commit.message().unwrap_or("").to_string();
        let (summary, description) = match message.split_once('\n') {
            Some((first, rest)) => (first.to_string(), rest.trim().to_string()),
            None => (message.trim().to_string(), String::new()),
        };

        commits.push(PRCommit {
            change_id,
            sha: oid.to_string(),
            summary,
            description,
        });
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;
    use test_repo::TestRepo;

    fn jj_change_id(dir: &str, sha: &str) -> ChangeId {
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
        ChangeId::try_from(s.trim()).unwrap()
    }

    #[test]
    fn reverse_hex_encode_matches_jj() {
        assert_eq!(
            reverse_hex_encode(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            "zyxwvutsrqponmlk"
        );
        assert_eq!(reverse_hex_encode(&[0x00; 8]), "zzzzzzzzzzzzzzzz");
        assert_eq!(reverse_hex_encode(&[0xff; 8]), "kkkkkkkkkkkkkkkk");
    }

    #[test]
    fn synthetic_change_id_matches_jj_for_git_commit() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("test.txt", "content\n").unwrap();
        let commit_id = repo.git_commit("pure git commit").unwrap();

        let jj_cid = jj_change_id(repo.path(), &commit_id.to_string());

        let our_change_id = synthetic_change_id(commit_id);

        assert_eq!(
            our_change_id, jj_cid,
            "synthetic_change_id should match jj's output for git-only commits"
        );
    }

    #[test]
    fn get_change_id_or_synthetic_uses_header_for_jj_commits() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("test.txt", "content\n").unwrap();
        let result = repo.commit("jj commit").unwrap();
        let commit_id = result.created.commit_id;
        let expected_change_id = result.created.change_id;

        let commit = repo.repo.find_commit(commit_id.oid()).unwrap();
        let change_id = get_change_id_or_synthetic(&commit);

        assert_eq!(
            change_id, expected_change_id,
            "For jj commits, should use the header change-id, not synthetic"
        );
    }

    #[test]
    fn get_commits_in_range_single_commit() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("base.txt", "base\n").unwrap();
        let base = repo.commit("base").unwrap().created.commit_id;
        repo.write_file("feature.txt", "feature\n").unwrap();
        let head = repo.commit("feature").unwrap().created.commit_id;

        let repository = Repository::open(repo.path()).unwrap();
        let commits = get_commits_in_range(&repository, base, head).unwrap();

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].sha, head.to_string());
        assert_eq!(commits[0].summary, "feature");
    }

    #[test]
    fn get_commits_in_range_newest_first() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("base.txt", "base\n").unwrap();
        let base = repo.commit("base").unwrap().created.commit_id;

        repo.write_file("a.txt", "a\n").unwrap();
        let sha1 = repo.commit("first").unwrap().created.commit_id;
        repo.write_file("b.txt", "b\n").unwrap();
        let sha2 = repo.commit("second").unwrap().created.commit_id;
        repo.write_file("c.txt", "c\n").unwrap();
        let sha3 = repo.commit("third").unwrap().created.commit_id;

        let repository = Repository::open(repo.path()).unwrap();
        let commits = get_commits_in_range(&repository, base, sha3).unwrap();

        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].sha, sha3.to_string());
        assert_eq!(commits[1].sha, sha2.to_string());
        assert_eq!(commits[2].sha, sha1.to_string());
    }

    #[test]
    fn get_commits_in_range_empty() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("file.txt", "content\n").unwrap();
        let sha = repo.commit("only").unwrap().created.commit_id;

        let repository = Repository::open(repo.path()).unwrap();
        let commits = get_commits_in_range(&repository, sha, sha).unwrap();
        assert_eq!(commits.len(), 0);
    }

    #[test]
    fn get_commits_in_range_divergent_excludes_other_branch() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("base.txt", "base\n").unwrap();
        let a = repo.commit("A").unwrap().created;

        repo.write_file("d.txt", "d\n").unwrap();
        let d = repo.commit("D").unwrap().created;
        repo.write_file("e.txt", "e\n").unwrap();
        let e = repo.commit("E").unwrap().created;

        repo.new_revision(a.change_id).unwrap();
        repo.write_file("b.txt", "b\n").unwrap();
        repo.commit("B").unwrap();
        repo.write_file("c.txt", "c\n").unwrap();
        repo.commit("C").unwrap();

        let repository = Repository::open(repo.path()).unwrap();
        let commits = get_commits_in_range(&repository, a.commit_id, e.commit_id).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].sha, e.commit_id.to_string());
        assert_eq!(commits[1].sha, d.commit_id.to_string());
    }
}
