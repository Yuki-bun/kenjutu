use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::models::{ChangeId, JjCommit, JjLogResult, JjStatus, PRCommit};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to run jj command: {0}")]
    Command(String),

    #[error("jj command failed: {0}")]
    JjFailed(String),

    #[error("Failed to parse output: {0}")]
    Parse(String),
}

const TEMPLATE: &str = r#"json(self) ++ "\n""#;

#[derive(Deserialize)]
struct JjSignature {
    name: String,
    email: String,
    timestamp: String,
}

#[derive(Deserialize)]
struct JjEntry {
    change_id: String,
    commit_id: String,
    description: String,
    author: JjSignature,
    immutable: bool,
    current_working_copy: bool,
    parents: Vec<String>,
}

static JJ_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn find_jj_executable() -> Option<PathBuf> {
    JJ_PATH
        .get_or_init(|| {
            let mut candidates: Vec<PathBuf> = vec![
                PathBuf::from("/opt/homebrew/bin/jj"),
                PathBuf::from("/usr/local/bin/jj"),
                PathBuf::from("/run/current-system/sw/bin/jj"),
            ];

            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join(".cargo/bin/jj"));
                candidates.push(home.join(".nix-profile/bin/jj"));
            }

            for path in &candidates {
                if path.exists() {
                    log::info!("Found jj executable at: {}", path.display());
                    return Some(path.clone());
                }
            }

            if Command::new("jj")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                log::info!("Found jj executable in PATH");
                return Some(PathBuf::from("jj"));
            }

            log::warn!("jj executable not found in any known location");
            None
        })
        .clone()
}

fn jj_command() -> Option<Command> {
    find_jj_executable().map(Command::new)
}

/// Check if jj CLI is installed
pub fn is_installed() -> bool {
    find_jj_executable().is_some()
}

/// Check if directory is a jj repository
pub fn is_jj_repo(local_dir: &str) -> bool {
    jj_command()
        .map(|mut cmd| {
            cmd.args(["root"])
                .current_dir(local_dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Get jj status for a directory
pub fn get_status(local_dir: &str) -> JjStatus {
    JjStatus {
        is_installed: is_installed(),
        is_jj_repo: is_jj_repo(local_dir),
    }
}

/// Get mutable commits + 1 ancestor using jj log
///
/// Outputs one JSON object per line (newline-delimited JSON) via `json(self)`.
pub fn get_log(local_dir: &str) -> Result<JjLogResult> {
    let mut cmd =
        jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args([
            "log",
            "--no-graph",
            "-r",
            "mutable() | ancestors(mutable(), 2)",
            "-T",
            TEMPLATE,
        ])
        .current_dir(local_dir)
        .output()
        .map_err(|e| Error::Command(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits = parse_log_output(&stdout)?;

    Ok(JjLogResult { commits })
}

/// Get commits in a range (base_sha..head_sha) using jj log
pub fn get_commits_in_range(
    local_dir: &str,
    base_sha: &str,
    head_sha: &str,
) -> Result<Vec<PRCommit>> {
    let revset = format!("{base_sha}..{head_sha}");

    let mut cmd =
        jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args(["log", "--no-graph", "-r", &revset, "-T", TEMPLATE])
        .current_dir(local_dir)
        .output()
        .map_err(|e| Error::Command(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_commits_in_range_output(&stdout)
}

fn split_description(full: &str) -> (String, String) {
    match full.split_once('\n') {
        Some((first, rest)) => (first.to_string(), rest.trim_start().to_string()),
        None => (full.trim().to_string(), String::new()),
    }
}

fn parse_commits_in_range_output(output: &str) -> Result<Vec<PRCommit>> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let entry: JjEntry =
                serde_json::from_str(line).map_err(|e| Error::Parse(e.to_string()))?;
            let (summary, description) = split_description(&entry.description);
            Ok(PRCommit {
                change_id: ChangeId::from(entry.change_id),
                sha: entry.commit_id,
                summary,
                description,
            })
        })
        .collect()
}

fn parse_log_output(output: &str) -> Result<Vec<JjCommit>> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let entry: JjEntry =
                serde_json::from_str(line).map_err(|e| Error::Parse(e.to_string()))?;
            let (summary, description) = split_description(&entry.description);
            Ok(JjCommit {
                change_id: ChangeId::from(entry.change_id),
                commit_id: entry.commit_id,
                summary,
                description,
                author: entry.author.name,
                email: entry.author.email,
                timestamp: entry.author.timestamp,
                is_immutable: entry.immutable,
                is_working_copy: entry.current_working_copy,
                parents: entry.parents.into_iter().map(ChangeId::from).collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    #[test]
    fn split_description_single_line() {
        let (summary, body) = split_description("just a summary");
        assert_eq!(summary, "just a summary");
        assert_eq!(body, "");
    }

    #[test]
    fn split_description_multiline() {
        let (summary, body) = split_description("summary line\n\nbody text here");
        assert_eq!(summary, "summary line");
        assert_eq!(body, "body text here");
    }

    #[test]
    fn split_description_trims_leading_blank_line() {
        let (summary, body) = split_description("summary\n\n  body");
        assert_eq!(summary, "summary");
        assert_eq!(body, "body");
    }

    #[test]
    fn log_returns_commits() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a\n").unwrap();
        repo.commit("first commit").unwrap();

        let result = get_log(repo.path()).unwrap();
        assert!(!result.commits.is_empty());
        let commit = result.commits.iter().find(|c| c.summary == "first commit");
        assert!(commit.is_some());
    }

    #[test]
    fn log_populates_author_fields() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a\n").unwrap();
        repo.commit("author test").unwrap();

        let result = get_log(repo.path()).unwrap();
        let commit = result
            .commits
            .iter()
            .find(|c| c.summary == "author test")
            .unwrap();
        assert_eq!(commit.author, "Test User");
        assert_eq!(commit.email, "test@test.com");
        assert!(!commit.timestamp.is_empty());
    }

    #[test]
    fn log_description_split_into_summary_and_body() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a\n").unwrap();
        repo.commit("summary line\n\nbody paragraph").unwrap();

        let result = get_log(repo.path()).unwrap();
        let commit = result
            .commits
            .iter()
            .find(|c| c.summary == "summary line")
            .unwrap();
        assert_eq!(commit.description, "body paragraph");
    }

    #[test]
    fn log_description_with_special_chars() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a\n").unwrap();
        repo.commit(r#"fix: handle "quoted" values & <tags>"#)
            .unwrap();

        let result = get_log(repo.path()).unwrap();
        let commit = result
            .commits
            .iter()
            .find(|c| c.summary.contains("quoted"))
            .unwrap();
        assert_eq!(commit.summary, r#"fix: handle "quoted" values & <tags>"#);
    }

    #[test]
    fn log_immutable_false_for_mutable_commits() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a\n").unwrap();
        repo.commit("mutable commit").unwrap();

        let result = get_log(repo.path()).unwrap();
        let commit = result
            .commits
            .iter()
            .find(|c| c.summary == "mutable commit")
            .unwrap();
        assert!(!commit.is_immutable);
    }

    #[test]
    fn commits_in_range_single_commit() {
        let repo = TestRepo::new().unwrap();

        repo.write_file("base.txt", "base\n").unwrap();
        let base_sha = repo.commit("base commit").unwrap().created.commit_id;

        repo.write_file("feature.txt", "feature\n").unwrap();
        let head_sha = repo.commit("feature commit").unwrap().created.commit_id;

        let commits = get_commits_in_range(repo.path(), &base_sha, &head_sha).unwrap();

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].sha, head_sha.to_string());
        assert_eq!(commits[0].summary, "feature commit");
        assert!(!commits[0].change_id.as_str().is_empty());
    }

    #[test]
    fn commits_in_range_multiple_commits() {
        let repo = TestRepo::new().unwrap();

        repo.write_file("base.txt", "base\n").unwrap();
        let base_sha = repo.commit("base commit").unwrap().created.commit_id;

        repo.write_file("a.txt", "a\n").unwrap();
        let sha1 = repo.commit("first").unwrap().created.commit_id;

        repo.write_file("b.txt", "b\n").unwrap();
        let sha2 = repo.commit("second").unwrap().created.commit_id;

        repo.write_file("c.txt", "c\n").unwrap();
        let sha3 = repo.commit("third").unwrap().created.commit_id;

        let commits = get_commits_in_range(repo.path(), &base_sha, &sha3).unwrap();

        assert_eq!(commits.len(), 3);
        // jj log returns newest first
        let shas: Vec<&str> = commits.iter().map(|c| c.sha.as_str()).collect();
        assert!(shas.contains(&sha1.as_str()));
        assert!(shas.contains(&sha2.as_str()));
        assert!(shas.contains(&sha3.as_str()));

        // Verify order: newest first
        assert_eq!(
            commits[0].sha, sha3,
            "First result should be newest (third)"
        );
        assert_eq!(
            commits[1].sha, sha2,
            "Second result should be middle (second)"
        );
        assert_eq!(
            commits[2].sha, sha1,
            "Third result should be oldest (first)"
        );
    }

    #[test]
    fn commits_in_range_empty_range() {
        let repo = TestRepo::new().unwrap();

        repo.write_file("file.txt", "content\n").unwrap();
        let sha = repo.commit("only commit").unwrap().created.commit_id;

        // Range from sha to sha should be empty
        let commits = get_commits_in_range(repo.path(), &sha, &sha).unwrap();
        assert_eq!(commits.len(), 0);
    }

    #[test]
    fn commits_in_range_divergent_history_excludes_other_branch() {
        let repo = TestRepo::new().unwrap();
        // E   C
        // |   |
        // D   B
        //  \ /
        //   A

        // Create base commit A
        repo.write_file("base.txt", "base\n").unwrap();
        let sha_a = repo.commit("commit A").unwrap().created.commit_id;

        // Create feature branch: D and E (children of A)
        repo.write_file("feature_d.txt", "d\n").unwrap();
        let sha_d = repo.commit("commit D").unwrap().created.commit_id;
        repo.write_file("feature_e.txt", "e\n").unwrap();
        let sha_e = repo.commit("commit E").unwrap().created.commit_id;

        // Create main branch: B and C (also children of A, diverged from feature)
        // Use commit_with_parents with single parent to specify parent explicitly
        repo.new_revision(&sha_a).unwrap();
        repo.commit("commit B").unwrap();
        repo.write_file("main_c.txt", "c\n").unwrap();
        repo.commit("commit C").unwrap();

        // Get commits from A to E (should only include feature branch)
        let commits =
            get_commits_in_range(repo.path(), &sha_a.to_string(), &sha_e.to_string()).unwrap();

        // Should only contain D and E, not B or C
        assert_eq!(commits.len(), 2, "Range A..E should only include D and E");

        assert_eq!(commits[0].sha, sha_e, "Should include commit E");
        assert_eq!(commits[1].sha, sha_d, "Should include commit D");
    }

    #[test]
    fn commits_in_range_invalid_sha_returns_error() {
        let repo = TestRepo::new().unwrap();

        repo.write_file("base.txt", "base\n").unwrap();
        let valid_sha = repo.commit("base").unwrap().created.commit_id;
        let invalid_sha = "nonexistent1234567890abcdef1234567890abcdef";

        // Test with invalid base SHA
        let result = get_commits_in_range(repo.path(), invalid_sha, &valid_sha.to_string());
        assert!(result.is_err(), "Should return error for invalid base SHA");
        let Error::JjFailed(_) = result.unwrap_err() else {
            panic!("Expected JjFailed error");
        };
        // Test with invalid head SHA
        let result = get_commits_in_range(repo.path(), &valid_sha.to_string(), invalid_sha);
        assert!(result.is_err(), "Should return error for invalid head SHA");
        let Error::JjFailed(_) = result.unwrap_err() else {
            panic!("Expected JjFailed error");
        };
    }
}
