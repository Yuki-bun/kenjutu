use std::process::Command;

use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, JjCommit, JjLogResult, JjStatus};

pub struct JjService;

impl JjService {
    /// Check if jj CLI is installed
    pub fn is_installed() -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if directory is a jj repository
    pub fn is_jj_repo(local_dir: &str) -> bool {
        Command::new("jj")
            .args(["root"])
            .current_dir(local_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get jj status for a directory
    pub fn get_status(local_dir: &str) -> JjStatus {
        JjStatus {
            is_installed: Self::is_installed(),
            is_jj_repo: Self::is_jj_repo(local_dir),
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

        let output = Command::new("jj")
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
            .map_err(|e| {
                log::error!("Failed to run jj log: {}", e);
                CommandError::bad_input("Failed to run jj command")
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("jj log failed: {}", stderr);
            return Err(CommandError::bad_input(format!(
                "jj log failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = Self::parse_log_output(&stdout)?;

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

            // Parse parent change_ids (comma-separated, may be empty)
            let parents: Vec<ChangeId> = parts[8]
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| ChangeId::from(s.to_string()))
                .collect();

            // Parse full description - it's JSON-escaped, so unescape it
            let full_description = serde_json::from_str::<String>(parts[2]).map_err(|e| {
                log::error!("Failed to parse description JSON: {}", e);
                CommandError::Internal
            })?;

            // Split into summary (first line) and description (rest)
            let (summary, description) = match full_description.split_once('\n') {
                Some((first, rest)) => (first.to_string(), rest.trim_start().to_string()),
                None => (full_description, String::new()),
            };

            commits.push(JjCommit {
                change_id: ChangeId::from(parts[0].to_string()),
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
}
