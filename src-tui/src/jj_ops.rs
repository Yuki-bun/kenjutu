use std::process::Command;

use anyhow::{Context, Result};
use kenjutu_types::ChangeId;

pub use kenjutu_core::services::jj::describe;

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

#[cfg(test)]
mod tests {
    use super::*;
    use kenjutu_core::models::GraphRow;
    use kenjutu_core::services::graph;
    use test_repo::TestRepo;

    /// Helper to extract commits from a CommitGraph
    fn get_commits(local_dir: &str) -> Vec<kenjutu_core::models::JjCommit> {
        let graph = graph::get_log_graph(local_dir).unwrap();
        graph
            .rows
            .into_iter()
            .filter_map(|row| match row {
                GraphRow::Commit(commit_row) => Some(commit_row.commit),
                GraphRow::Elision(_) => None,
            })
            .collect()
    }

    #[test]
    fn describe_updates_summary() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("test.txt", "hello").unwrap();
        repo.commit("original message").unwrap();

        let commits = get_commits(repo.path());
        let committed = commits
            .iter()
            .find(|c| c.summary == "original message")
            .unwrap();
        let change_id = committed.change_id;

        describe(repo.path(), change_id, "updated message").unwrap();

        let commits = get_commits(repo.path());
        let updated = commits.iter().find(|c| c.change_id == change_id).unwrap();
        assert_eq!(updated.summary, "updated message");
    }

    #[test]
    fn describe_working_copy() {
        let repo = TestRepo::new().unwrap();
        let commits = get_commits(repo.path());
        let wc = commits.iter().find(|c| c.is_working_copy).unwrap();

        describe(repo.path(), wc.change_id, "wc description").unwrap();

        let commits = get_commits(repo.path());
        let wc = commits.iter().find(|c| c.is_working_copy).unwrap();
        assert_eq!(wc.summary, "wc description");
    }

    #[test]
    fn new_on_top_creates_child_commit() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("base commit").unwrap();

        let commits = get_commits(repo.path());
        let base = commits.iter().find(|c| c.summary == "base commit").unwrap();
        let base_change_id = base.change_id;

        new_on_top(repo.path(), &base_change_id).unwrap();

        let commits = get_commits(repo.path());
        let wc = commits.iter().find(|c| c.is_working_copy).unwrap();
        assert_eq!(wc.summary, "");
        assert_ne!(wc.change_id, base_change_id);
    }
}
