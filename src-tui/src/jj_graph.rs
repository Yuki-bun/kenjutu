use std::process::Command;

use anyhow::{Context, Result};
use kenjutu_types::ChangeId;

#[derive(Clone, Debug)]
pub struct GraphCommit {
    pub change_id: ChangeId,
    pub commit_id: String,
    pub summary: String,
    pub author: String,
    pub is_working_copy: bool,
    pub is_immutable: bool,
}

#[derive(Clone, Debug)]
pub struct JjGraphEntry {
    pub commit: GraphCommit,
    /// Graph characters for the commit's node line (e.g. "○  ", "│ @  ")
    pub gutter: String,
    /// Graph-only continuation lines after this commit (e.g. "├─╯", "│")
    pub continuation_lines: Vec<String>,
}

/// Fetch commits with jj-rendered graph lines.
///
/// Calls `jj log` without `--no-graph` so jj generates the graph gutter.
/// A `\x01` marker in the template separates graph gutter from structured data.
pub fn get_log_with_graph(local_dir: &str) -> Result<Vec<JjGraphEntry>> {
    let template = r#""\x01" ++ change_id ++ "\x00" ++ commit_id ++ "\x00" ++ description.first_line() ++ "\x00" ++ author.name() ++ "\x00" ++ immutable ++ "\x00" ++ current_working_copy"#;

    let output = Command::new("jj")
        .args([
            "log",
            "--color",
            "never",
            "-r",
            "mutable() | ancestors(mutable(), 2)",
            "-T",
            template,
        ])
        .current_dir(local_dir)
        .output()
        .context("Failed to execute jj log command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "jj log command failed with status {}: {}",
            output.status,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_graph_output(&stdout)
}

/// Describe (set the commit message of) a jj revision.
pub fn describe(local_dir: &str, change_id: &ChangeId, message: &str) -> Result<()> {
    let change_id_str = change_id.to_string();
    let output = Command::new("jj")
        .args(["describe", "-r", &change_id_str, "-m", message])
        .current_dir(local_dir)
        .output()
        .context("Failed to execute jj describe command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "jj describe failed with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    Ok(())
}

/// Create a new empty commit on top of the given revision.
///
/// Runs `jj new -r <change_id>`, which creates a new working-copy commit
/// whose parent is the specified revision.
pub fn new_on_top(local_dir: &str, change_id: &ChangeId) -> Result<()> {
    let change_id_str = change_id.to_string();
    let output = Command::new("jj")
        .args(["new", "-r", &change_id_str])
        .current_dir(local_dir)
        .output()
        .context("Failed to execute jj new command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "jj new failed with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    Ok(())
}

fn parse_graph_output(output: &str) -> Result<Vec<JjGraphEntry>> {
    let mut entries: Vec<JjGraphEntry> = Vec::new();

    for line in output.lines() {
        if let Some(marker_pos) = line.find('\x01') {
            let gutter = line[..marker_pos].to_string();
            let data = &line[marker_pos + 1..];

            let parts: Vec<&str> = data.split('\x00').collect();
            if parts.len() < 6 {
                log::warn!(
                    "Skipping malformed graph log line (expected 6 fields, got {})",
                    parts.len()
                );
                continue;
            }

            let change_id =
                ChangeId::try_from(parts[0]).context("Invalid ChangeId in graph log output")?;

            let commit = GraphCommit {
                change_id,
                commit_id: parts[1].to_string(),
                summary: parts[2].to_string(),
                author: parts[3].to_string(),
                is_immutable: parts[4] == "true",
                is_working_copy: parts[5] == "true",
            };

            entries.push(JjGraphEntry {
                commit,
                gutter,
                continuation_lines: Vec::new(),
            });
        } else {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "~" {
                continue;
            }

            if let Some(last) = entries.last_mut() {
                last.continuation_lines.push(line.to_string());
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    #[test]
    fn working_copy_is_included() {
        let repo = TestRepo::new().unwrap();
        let entries = get_log_with_graph(repo.path()).unwrap();

        let wc = entries.iter().find(|e| e.commit.is_working_copy);
        assert!(wc.is_some(), "working copy should be in the output");
        assert!(
            wc.unwrap().gutter.contains('@'),
            "working copy gutter should contain @"
        );
    }

    #[test]
    fn empty_description_is_preserved() {
        let repo = TestRepo::new().unwrap();
        let entries = get_log_with_graph(repo.path()).unwrap();

        let wc = entries.iter().find(|e| e.commit.is_working_copy).unwrap();
        // Working copy starts with no description
        assert_eq!(wc.commit.summary, "");
    }

    #[test]
    fn committed_revision_appears() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("test.txt", "hello").unwrap();
        repo.commit("add test file").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        let committed = entries.iter().find(|e| e.commit.summary == "add test file");
        assert!(committed.is_some(), "committed revision should appear");
        assert!(!committed.unwrap().commit.is_working_copy);
    }

    #[test]
    fn immutable_root_is_included() {
        let repo = TestRepo::new().unwrap();
        let entries = get_log_with_graph(repo.path()).unwrap();

        let immutable = entries.iter().find(|e| e.commit.is_immutable);
        assert!(
            immutable.is_some(),
            "immutable root commit should be included"
        );
    }

    #[test]
    fn multiple_commits_produce_graph_lines() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("first").unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("second").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        // Should have at least: working copy, "second", "first", root
        assert!(entries.len() >= 4);

        // At least some entries should have non-empty gutters with graph chars
        let has_graph = entries
            .iter()
            .any(|e| e.gutter.contains('○') || e.gutter.contains('@') || e.gutter.contains('◆'));
        assert!(
            has_graph,
            "entries should have graph node characters in gutters"
        );
    }

    #[test]
    fn describe_updates_summary() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("test.txt", "hello").unwrap();
        repo.commit("original message").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        let committed = entries
            .iter()
            .find(|e| e.commit.summary == "original message")
            .unwrap();
        let change_id = committed.commit.change_id;

        describe(repo.path(), &change_id, "updated message").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        let updated = entries
            .iter()
            .find(|e| e.commit.change_id == change_id)
            .unwrap();
        assert_eq!(updated.commit.summary, "updated message");
    }

    #[test]
    fn describe_working_copy() {
        let repo = TestRepo::new().unwrap();
        let entries = get_log_with_graph(repo.path()).unwrap();
        let wc = entries.iter().find(|e| e.commit.is_working_copy).unwrap();

        describe(repo.path(), &wc.commit.change_id, "wc description").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        let wc = entries.iter().find(|e| e.commit.is_working_copy).unwrap();
        assert_eq!(wc.commit.summary, "wc description");
    }

    #[test]
    fn new_on_top_creates_child_commit() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("base commit").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        let base = entries
            .iter()
            .find(|e| e.commit.summary == "base commit")
            .unwrap();
        let base_change_id = base.commit.change_id;

        new_on_top(repo.path(), &base_change_id).unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        // The new working copy should be at the top (first entry) with an empty summary
        let wc = entries.iter().find(|e| e.commit.is_working_copy).unwrap();
        assert_eq!(wc.commit.summary, "");
        // The new working copy should be different from the base commit
        assert_ne!(wc.commit.change_id, base_change_id);
    }

    #[test]
    fn branching_produces_continuation_lines() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        let first = repo.commit("first").unwrap();

        // Create a branch from "first"
        repo.new_revision(first.created.change_id).unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("branch-a").unwrap();

        repo.new_revision(first.created.change_id).unwrap();
        repo.write_file("c.txt", "c").unwrap();
        repo.commit("branch-b").unwrap();

        let entries = get_log_with_graph(repo.path()).unwrap();
        // With branching, jj should produce continuation lines between some entries
        let total_continuations: usize = entries.iter().map(|e| e.continuation_lines.len()).sum();
        assert!(
            total_continuations > 0,
            "branching should produce continuation graph lines"
        );
    }
}
