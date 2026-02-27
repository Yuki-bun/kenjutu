use std::path::PathBuf;

use git2::{
    AutotagOption, Commit, Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository,
};

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

    #[error("SSH authentication failed: {0}")]
    SshAuth(String),
}

#[derive(Debug, Clone)]
pub enum SshCredential {
    Agent,
    KeyFile(PathBuf),
}

/// Provides an ordered list of SSH credentials to try when authenticating.
pub trait SshCredentialProvider {
    fn ssh_credentials(&self) -> Vec<SshCredential>;
}

pub fn open_repository(local_dir: &str) -> Result<Repository> {
    Repository::open(local_dir).map_err(|_| Error::RepoNotFound(local_dir.to_string()))
}

/// Falls back to "origin" if no remotes match
fn find_remote_by_url<'r>(
    repo: &'r Repository,
    remote_urls: &[&str],
) -> std::result::Result<git2::Remote<'r>, git2::Error> {
    fn normalize(url: &str) -> &str {
        url.strip_suffix(".git").unwrap_or(url)
    }

    let candidates: Vec<&str> = remote_urls.iter().map(|u| normalize(u)).collect();

    let remotes = repo.remotes()?;
    for name in remotes.iter().flatten() {
        if let Ok(remote) = repo.find_remote(name) {
            if let Some(url) = remote.url() {
                if candidates.contains(&normalize(url)) {
                    return repo.find_remote(name);
                }
            }
        }
    }

    repo.find_remote("origin")
}

pub fn get_or_fetch_commit<'r>(
    repo: &'r Repository,
    commit_id: CommitId,
    remote_urls: &[&str],
    cred_provider: &dyn SshCredentialProvider,
) -> Result<Commit<'r>> {
    let oid = commit_id.oid();
    if let Ok(commit) = repo.find_commit(oid) {
        return Ok(commit);
    }

    let mut remote = find_remote_by_url(repo, remote_urls)?;
    log::info!(
        "Commit {} not found locally, fetching from remote '{}'",
        oid,
        remote.name().unwrap_or("<unknown>")
    );

    let callbacks = build_remote_callbacks(repo, cred_provider);
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);
    fo.download_tags(AutotagOption::None);

    let refspec = format!("{}:", oid);

    remote
        .fetch(&[&refspec], Some(&mut fo), None)
        .map_err(|e| {
            if e.class() == git2::ErrorClass::Ssh || e.code() == git2::ErrorCode::Auth {
                let mut msg = format!("Failed to authenticate with remote: {}", e.message());
                msg.push_str("\n\nTroubleshooting:");
                msg.push_str("\n  - Ensure your SSH agent is running (`ssh-add -l`)");
                msg.push_str("\n  - Or configure an SSH key path in Settings");
                Error::SshAuth(msg)
            } else {
                Error::Git2(e)
            }
        })?;

    repo.find_commit(oid)
        .map_err(|_| Error::CommitNotFound(oid.to_string()))
}

/// Iterates SSH credentials from the provider, then falls back to HTTPS helpers.
fn build_remote_callbacks<'a>(
    repo: &'a Repository,
    cred_provider: &dyn SshCredentialProvider,
) -> RemoteCallbacks<'a> {
    let credentials = cred_provider.ssh_credentials();
    let mut idx = 0;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        let username = username_from_url.unwrap_or("git");

        if allowed.contains(CredentialType::SSH_KEY) {
            if idx >= credentials.len() {
                return Err(git2::Error::from_str("all SSH methods exhausted"));
            }
            let cred = match &credentials[idx] {
                SshCredential::Agent => {
                    log::info!("SSH auth: trying agent");
                    Cred::ssh_key_from_agent(username)
                }
                SshCredential::KeyFile(path) => {
                    log::info!("SSH auth: trying {:?}", path);
                    Cred::ssh_key(username, None, path, None)
                }
            };
            idx += 1;
            return cred;
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            let config = repo.config().or_else(|_| git2::Config::open_default())?;
            return Cred::credential_helper(&config, url, username_from_url);
        }

        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(git2::Error::from_str("no auth methods available"))
    });

    callbacks
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
    fn find_remote_by_url_matches_exact() {
        let repo = TestRepo::new().unwrap();
        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo
            .remote("upstream", "https://github.com/octocat/Hello-World.git")
            .unwrap();

        let remote =
            find_remote_by_url(&git_repo, &["https://github.com/octocat/Hello-World.git"]).unwrap();
        assert_eq!(remote.name(), Some("upstream"));
    }

    #[test]
    fn find_remote_by_url_strips_git_suffix() {
        let repo = TestRepo::new().unwrap();
        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo
            .remote("upstream", "https://github.com/octocat/Hello-World")
            .unwrap();

        let remote =
            find_remote_by_url(&git_repo, &["https://github.com/octocat/Hello-World.git"]).unwrap();
        assert_eq!(remote.name(), Some("upstream"));
    }

    #[test]
    fn find_remote_by_url_matches_ssh() {
        let repo = TestRepo::new().unwrap();
        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo
            .remote("mine", "git@github.com:octocat/Hello-World.git")
            .unwrap();

        let remote = find_remote_by_url(
            &git_repo,
            &[
                "https://github.com/octocat/Hello-World.git",
                "git@github.com:octocat/Hello-World.git",
            ],
        )
        .unwrap();
        assert_eq!(remote.name(), Some("mine"));
    }

    #[test]
    fn find_remote_by_url_falls_back_to_origin() {
        let repo = TestRepo::new().unwrap();
        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo
            .remote("origin", "https://github.com/default/origin-repo.git")
            .unwrap();
        git_repo
            .remote("other", "https://github.com/other/repo.git")
            .unwrap();

        let remote =
            find_remote_by_url(&git_repo, &["https://github.com/no-match/nowhere.git"]).unwrap();
        assert_eq!(remote.name(), Some("origin"));
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
