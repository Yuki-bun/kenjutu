use std::collections::HashMap;
use std::path::Path;

use kenjutu_core::models::JjCommit;
use kenjutu_core::services::jj::{self, Error};
use kenjutu_types::ChangeId;
use ratatui::text::Line;

use crate::ansi;

/// Template based on jj's `builtin_log_compact` that preserves native colored output
/// while embedding structured data after a \x01 marker on commit header lines.
///
/// Embedded fields (after \x01, separated by \x00):
///   change_id, commit_id, immutable, current_working_copy, description.first_line()
const TEMPLATE: &str = concat!(
    "if(self.root(),",
    "  format_root_commit(self)",
    "    ++ \"\\x01\" ++ change_id ++ \"\\x00\" ++ commit_id",
    "    ++ \"\\x00\" ++ immutable ++ \"\\x00\" ++ current_working_copy",
    "    ++ \"\\x00\" ++ description.first_line()",
    "    ++ \"\\n\",",
    "  label(",
    "    separate(\" \",",
    "      if(self.current_working_copy(), \"working_copy\"),",
    "      if(self.immutable(), \"immutable\", \"mutable\"),",
    "      if(self.conflict(), \"conflicted\"),",
    "    ),",
    "    concat(",
    "      format_short_commit_header(self)",
    "        ++ \"\\x01\" ++ change_id ++ \"\\x00\" ++ commit_id",
    "        ++ \"\\x00\" ++ immutable ++ \"\\x00\" ++ current_working_copy",
    "        ++ \"\\x00\" ++ description.first_line()",
    "        ++ \"\\n\",",
    "      separate(\" \",",
    "        if(self.empty(), empty_commit_marker),",
    "        if(self.description(),",
    "          self.description().first_line(),",
    "          label(if(self.empty(), \"empty\"), description_placeholder),",
    "        ),",
    "      ) ++ \"\\n\",",
    "    ),",
    "  )",
    ")",
);

const REVSET: &str = "mutable() | ancestors(mutable(), 2)";

/// Minimal commit data extracted from the embedded marker.
/// Enough information for the TUI to support describe, new, and enter-review.
#[derive(Clone, Debug)]
pub struct LogCommit {
    pub change_id: ChangeId,
    pub commit_id: String,
    pub summary: String,
    pub is_immutable: bool,
    pub is_working_copy: bool,
}

impl LogCommit {
    /// Convert to a full `JjCommit` (with default values for unused fields).
    pub fn to_jj_commit(&self) -> JjCommit {
        JjCommit {
            change_id: self.change_id,
            commit_id: self.commit_id.clone(),
            summary: self.summary.clone(),
            description: String::new(),
            author: String::new(),
            email: String::new(),
            timestamp: String::new(),
            is_immutable: self.is_immutable,
            is_working_copy: self.is_working_copy,
            parents: Vec::new(),
        }
    }
}

/// The result of parsing `jj log --color always` output.
pub struct JjLogOutput {
    /// Display lines with ANSI colors parsed into Ratatui spans.
    pub lines: Vec<Line<'static>>,
    /// Maps line index → commit data (only for commit header lines).
    pub commits_by_line: HashMap<usize, LogCommit>,
    /// Sorted list of line indices that are commit header lines (for j/k navigation).
    pub commit_lines: Vec<usize>,
}

/// Run `jj log --color always` and parse the output into styled lines
/// with embedded commit metadata.
pub fn get_jj_log(local_dir: &Path) -> jj::Result<JjLogOutput> {
    let mut cmd =
        jj::jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;

    let output = cmd
        .args([
            "log",
            "--color",
            "always",
            "--no-pager",
            "-r",
            REVSET,
            "-T",
            TEMPLATE,
        ])
        .current_dir(local_dir)
        .output()
        .map_err(|e| Error::Command(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjFailed(ansi::strip_ansi(stderr.trim())));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_log_output(&stdout)
}

fn parse_log_output(stdout: &str) -> jj::Result<JjLogOutput> {
    let mut lines = Vec::new();
    let mut commits_by_line = HashMap::new();
    let mut commit_lines = Vec::new();

    for raw in stdout.lines() {
        // Check for \x01 marker (commit header lines)
        if let Some(marker_pos) = raw.find('\x01') {
            let display_raw = &raw[..marker_pos];
            let data_raw = &raw[marker_pos + 1..];

            // Strip ANSI from data portion before splitting
            let data_plain = ansi::strip_ansi(data_raw);
            let fields: Vec<&str> = data_plain.split('\x00').collect();

            let change_id_str = fields.first().copied().unwrap_or("");
            let commit_id = fields.get(1).copied().unwrap_or("").to_string();
            let is_immutable = fields.get(2).copied().unwrap_or("") == "true";
            let is_working_copy = fields.get(3).copied().unwrap_or("") == "true";
            let summary = fields.get(4).copied().unwrap_or("").to_string();

            let change_id: ChangeId = change_id_str
                .parse()
                .map_err(|e: kenjutu_types::InvalidChangeIdError| Error::Parse(e.to_string()))?;

            let line = ansi::parse_ansi_line(display_raw);
            let idx = lines.len();
            lines.push(line);

            commits_by_line.insert(
                idx,
                LogCommit {
                    change_id,
                    commit_id,
                    summary,
                    is_immutable,
                    is_working_copy,
                },
            );
            commit_lines.push(idx);
        } else if !raw.trim().is_empty() {
            // Non-commit lines (description, graph continuation, etc.)
            lines.push(ansi::parse_ansi_line(raw));
        }
    }

    Ok(JjLogOutput {
        lines,
        commits_by_line,
        commit_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ChangeId must be exactly 32 characters
    const CHANGE_ID: &str = "abcdefghijklmnopqrstuvwxyzabcdef";

    #[test]
    fn parse_plain_log_line() {
        let input = format!(
            "│ ○  \x1b[1;35mxy\x1b[0m\x1b[38;5;5mzzzzzz\x1b[0m user@test 2024-01-01\x01{CHANGE_ID}\x00aabbccdd1122334455\x00false\x00true\x00my summary"
        );
        let result = parse_log_output(&input).unwrap();
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.commit_lines.len(), 1);
        assert_eq!(result.commit_lines[0], 0);

        let commit = &result.commits_by_line[&0];
        assert_eq!(commit.commit_id, "aabbccdd1122334455");
        assert_eq!(commit.summary, "my summary");
        assert!(!commit.is_immutable);
        assert!(commit.is_working_copy);
    }

    #[test]
    fn parse_continuation_lines() {
        let input = format!(
            "│ ○  header\x01{CHANGE_ID}\x00bbbb\x00false\x00false\x00desc\n│    (empty) desc line\n│"
        );
        let result = parse_log_output(&input).unwrap();
        assert_eq!(result.lines.len(), 3); // header + desc line + graph continuation "│"
        assert_eq!(result.commit_lines.len(), 1);
    }
}
