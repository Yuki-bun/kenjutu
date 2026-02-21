use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::models::{ChangeId, InvalidChangeIdError, JjCommit, JjLogResult, JjStatus, PRCommit};

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

impl From<InvalidChangeIdError> for Error {
    fn from(err: InvalidChangeIdError) -> Self {
        Error::Parse(err.to_string())
    }
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
/// Uses null-byte (\x00) as field separator for safe parsing.
/// The description field uses .escape_json() to handle special characters
/// (quotes, newlines, etc.) safely.
pub fn get_log(local_dir: &str) -> Result<JjLogResult> {
    // Template outputs 9 fields separated by null bytes:
    // 1. change_id
    // 2. commit_id
    // 3. full description (JSON-escaped for safe parsing)
    // 4. author name
    // 5. author email
    // 6. author timestamp
    // 7. immutable (true/false)
    // 8. current_working_copy (true/false)
    // 9. parent change_ids (comma-separated, supports multiple parents for merges)
    let template = r#"separate("\x00",
            change_id,
            commit_id,
            description.escape_json(),
            author.name(),
            author.email(),
            author.timestamp(),
            immutable,
            current_working_copy,
            parents.map(|p| p.change_id()).join(",")
        ) ++ "\n""#;

    let mut cmd =
        jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args([
            "log",
            "--no-graph",
            "-r",
            "mutable() | ancestors(mutable(), 2)",
            "-T",
            template,
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
    let template = r#"separate("\x00",
            change_id,
            commit_id,
            description.escape_json()
        ) ++ "\n""#;

    let revset = format!("{base_sha}..{head_sha}");

    let mut cmd =
        jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args(["log", "--no-graph", "-r", &revset, "-T", template])
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

fn parse_commits_in_range_output(output: &str) -> Result<Vec<PRCommit>> {
    let mut commits = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\x00').collect();
        if parts.len() < 3 {
            log::warn!(
                "Skipping malformed jj log line (expected 3 fields, got {})",
                parts.len()
            );
            continue;
        }

        let full_description =
            serde_json::from_str::<String>(parts[2]).map_err(|e| Error::Parse(e.to_string()))?;

        let (summary, description) = match full_description.split_once('\n') {
            Some((first, rest)) => (first.to_string(), rest.trim().to_string()),
            None => (full_description.trim().to_string(), String::new()),
        };
        let change_id = ChangeId::try_from(parts[0]).map_err(|e| Error::Parse(e.to_string()))?;

        commits.push(PRCommit {
            change_id,
            sha: parts[1].to_string(),
            summary,
            description,
        });
    }

    Ok(commits)
}

pub fn get_change_id(local_dir: &Path, sha: &str) -> Result<ChangeId> {
    let template = r#"change_id"#;
    let revset = sha;

    let mut cmd =
        jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args(["log", "--no-graph", "-r", revset, "-T", template])
        .current_dir(local_dir)
        .output()
        .map_err(|e| Error::Command(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let change_id_str = stdout.trim();
    if change_id_str.is_empty() {
        Err(Error::JjFailed(format!(
            "No change_id found for commit {}. Output: {}",
            sha, stdout
        )))
    } else {
        Ok(ChangeId::try_from(change_id_str).map_err(|e| Error::Parse(e.to_string()))?)
    }
}

fn parse_log_output(output: &str) -> Result<Vec<JjCommit>> {
    let mut commits = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\x00').collect();
        if parts.len() < 9 {
            log::warn!(
                "Skipping malformed jj log line (expected 9 fields, got {})",
                parts.len()
            );
            continue;
        }

        let parents: Vec<ChangeId> = parts[8]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| ChangeId::try_from(s).map_err(Error::from))
            .collect::<Result<Vec<ChangeId>>>()?;

        // Parse full description - it's JSON-escaped, so unescape it
        let full_description =
            serde_json::from_str::<String>(parts[2]).map_err(|e| Error::Parse(e.to_string()))?;

        // Split into summary (first line) and description (rest)
        let (summary, description) = match full_description.split_once('\n') {
            Some((first, rest)) => (first.to_string(), rest.trim_start().to_string()),
            None => (full_description, String::new()),
        };
        let change_id = ChangeId::try_from(parts[0])?;

        commits.push(JjCommit {
            change_id,
            commit_id: parts[1].to_string(),
            summary,
            description,
            author: parts[3].to_string(),
            email: parts[4].to_string(),
            timestamp: parts[5].to_string(),
            is_immutable: parts[6] == "true",
            is_working_copy: parts[7] == "true",
            parents,
        });
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

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

    #[test]
    fn can_issue_change_id() {
        let repo = TestRepo::new().unwrap();

        repo.write_file("file.txt", "content\n").unwrap();
        let oid = repo.git_commit("test commit").unwrap();
        let result = get_change_id(&PathBuf::from(repo.path()), &oid.to_string());
        assert!(result.is_ok(), "Should successfully get change_id");
    }
}
