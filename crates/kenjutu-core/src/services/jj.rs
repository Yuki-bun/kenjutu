use kenjutu_types::{ChangeId, InvalidChangeIdError};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use crate::models::{JjCommit, JjLogResult, JjStatus};

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

