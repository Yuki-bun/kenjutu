use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::Repository;
use kenjutu_core::models::{FileChangeStatus, FileEntry, FileDiff};
use kenjutu_core::services::diff;
use kenjutu_types::CommitId;

/// A commit in the graph with parent edges
#[derive(Clone, Debug)]
pub struct GraphCommit {
    pub commit_id: CommitId,
    pub summary: String,
    #[allow(dead_code)]
    pub author: String,
    pub short_id: String,
    pub parent_ids: Vec<CommitId>,
}

/// Loads commits from the repository using git2 (not jj)
pub fn load_commits(repo: &Repository, max_count: usize) -> Result<Vec<GraphCommit>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head().context("No HEAD found")?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= max_count {
            break;
        }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let commit_id = CommitId::from(oid);
        let summary = commit
            .summary()
            .unwrap_or("(no message)")
            .to_string();
        let author = commit
            .author()
            .name()
            .unwrap_or("unknown")
            .to_string();
        let short_id = oid.to_string()[..8].to_string();
        let parent_ids: Vec<CommitId> = commit
            .parent_ids()
            .map(CommitId::from)
            .collect();

        commits.push(GraphCommit {
            commit_id,
            summary,
            author,
            short_id,
            parent_ids,
        });
    }

    Ok(commits)
}

/// Load the file list for a given commit
pub fn load_file_list(repo: &Repository, commit_id: CommitId) -> Result<Vec<FileEntry>> {
    let commit = repo.find_commit(commit_id.oid())?;
    let commit_tree = commit.tree()?;

    let parent_tree = if commit.parent_count() > 0 {
        commit.parent(0).ok().and_then(|p| p.tree().ok())
    } else {
        None
    };

    let mut opts = git2::DiffOptions::new();
    opts.context_lines(3)
        .interhunk_lines(0)
        .ignore_whitespace(false);

    let diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        Some(&mut opts),
    )?;

    let mut find_opts = git2::DiffFindOptions::new();
    find_opts.renames(true);
    let mut diff = diff;
    diff.find_similar(Some(&mut find_opts))?;

    let mut files = Vec::new();
    for (delta_idx, delta) in diff.deltas().enumerate() {
        let old_file = delta.old_file();
        let new_file = delta.new_file();
        let is_deletion = delta.status() == git2::Delta::Deleted;

        let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
        let new_path = if is_deletion {
            None
        } else {
            new_file.path().map(|p| p.to_string_lossy().to_string())
        };

        let status = match delta.status() {
            git2::Delta::Added => FileChangeStatus::Added,
            git2::Delta::Deleted => FileChangeStatus::Deleted,
            git2::Delta::Modified => FileChangeStatus::Modified,
            git2::Delta::Renamed => FileChangeStatus::Renamed,
            git2::Delta::Copied => FileChangeStatus::Copied,
            git2::Delta::Typechange => FileChangeStatus::Typechange,
            _ => FileChangeStatus::Modified,
        };

        let is_binary = old_file.is_binary() || new_file.is_binary();

        let (additions, deletions) = if let Some(patch) = git2::Patch::from_diff(&diff, delta_idx)? {
            let (_ctx, add, del) = patch.line_stats()?;
            (add as u32, del as u32)
        } else {
            (0, 0)
        };

        files.push(FileEntry {
            old_path,
            new_path,
            status,
            additions,
            deletions,
            is_binary,
            review_status: kenjutu_core::models::ReviewStatus::Unreviewed,
        });
    }

    Ok(files)
}

/// Load the diff for a specific file in a commit
pub fn load_file_diff(
    repo: &Repository,
    commit_id: CommitId,
    file_path: &str,
    old_path: Option<&str>,
) -> Result<FileDiff> {
    let path = PathBuf::from(file_path);
    let old = old_path.map(PathBuf::from);
    let file_diff = diff::generate_single_file_diff(
        repo,
        commit_id,
        &path,
        old.as_deref(),
    )?;
    Ok(file_diff)
}

/// Compute graph columns for rendering a commit graph.
/// Returns a vec of (column_index, connector_lines) for each commit.
pub fn compute_graph_layout(commits: &[GraphCommit]) -> Vec<GraphRow> {
    let mut id_to_idx: HashMap<CommitId, usize> = HashMap::new();
    for (i, c) in commits.iter().enumerate() {
        id_to_idx.insert(c.commit_id, i);
    }

    let mut active_lanes: Vec<Option<CommitId>> = Vec::new();
    let mut rows = Vec::new();

    for commit in commits {
        // Find which lane this commit occupies (if it's expected by a parent pointer)
        let col = active_lanes
            .iter()
            .position(|lane| *lane == Some(commit.commit_id))
            .unwrap_or_else(|| {
                // No lane reserved, take a free slot or add new
                let free = active_lanes.iter().position(|l| l.is_none());
                match free {
                    Some(idx) => idx,
                    None => {
                        active_lanes.push(None);
                        active_lanes.len() - 1
                    }
                }
            });

        // This commit now occupies this lane
        active_lanes[col] = None;

        // Connect parents: first parent takes the same lane
        let mut edges = Vec::new();
        for (pi, parent_id) in commit.parent_ids.iter().enumerate() {
            if id_to_idx.contains_key(parent_id) {
                if pi == 0 {
                    // First parent: continue in same lane
                    active_lanes[col] = Some(*parent_id);
                    edges.push((col, col));
                } else {
                    // Merge parent: find or create a lane
                    let existing = active_lanes
                        .iter()
                        .position(|l| *l == Some(*parent_id));
                    let target = existing.unwrap_or_else(|| {
                        let free = active_lanes.iter().position(|l| l.is_none());
                        match free {
                            Some(idx) => {
                                active_lanes[idx] = Some(*parent_id);
                                idx
                            }
                            None => {
                                active_lanes.push(Some(*parent_id));
                                active_lanes.len() - 1
                            }
                        }
                    });
                    edges.push((col, target));
                }
            }
        }

        // Trim trailing None lanes
        while active_lanes.last() == Some(&None) {
            active_lanes.pop();
        }

        rows.push(GraphRow {
            col,
            num_lanes: active_lanes.len(),
            edges,
        });
    }

    rows
}

#[derive(Clone, Debug)]
pub struct GraphRow {
    /// Which column this commit's node sits in
    pub col: usize,
    /// How many active lanes at this row
    pub num_lanes: usize,
    /// Edges from this node: (from_col, to_col)
    pub edges: Vec<(usize, usize)>,
}
