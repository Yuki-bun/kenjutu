use git2::{Delta, DiffLineType as Git2DiffLineType, Oid};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::db::DB;
use crate::errors::{CommandError, Result};
use crate::models::{DiffHunk, DiffLine, DiffLineType, FileChangeStatus, FileDiff, PatchId};
use crate::services::ReviewService;

pub struct DiffService;

impl DiffService {
    pub fn generate_diff(
        repository: &git2::Repository,
        commit_sha: &str,
        db: &mut DB,
        github_node_id: &str,
        pr_number: u64,
    ) -> Result<(Option<String>, Vec<FileDiff>)> {
        // Find commit
        let oid = Oid::from_str(commit_sha).map_err(|err| {
            log::error!("Invalid commit SHA: {err}");
            CommandError::bad_input("Invalid commit SHA")
        })?;

        let commit = repository.find_commit(oid).map_err(|err| {
            log::error!("Could not find commit: {err}");
            CommandError::Internal
        })?;

        // Extract change_id from commit
        let change_id = commit
            .header_field_bytes("change-id")
            .ok()
            .and_then(|buf| buf.as_str().map(String::from));

        // Get commit tree and parent tree
        let commit_tree = commit.tree().map_err(|err| {
            log::error!("Could not get commit tree: {err}");
            CommandError::Internal
        })?;

        let parent_tree = if commit.parent_count() > 0 {
            let parent = commit.parent(0).map_err(|err| {
                log::error!("Could not get parent commit: {err}");
                CommandError::Internal
            })?;
            Some(parent.tree().map_err(|err| {
                log::error!("Could not get parent tree: {err}");
                CommandError::Internal
            })?)
        } else {
            None
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts
            .context_lines(3)
            .interhunk_lines(0)
            .ignore_whitespace(false);

        let diff = repository
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )
            .map_err(|err| {
                log::error!("Failed to generate diff: {err}");
                CommandError::Internal
            })?;

        let reviewd_files =
            Self::get_reviewd_files(change_id.clone(), db, github_node_id, pr_number)?;

        // Process all file patches
        let mut files: Vec<FileDiff> = Vec::new();
        for (delta_idx, _) in diff.deltas().enumerate() {
            let patch = git2::Patch::from_diff(&diff, delta_idx).map_err(|err| {
                log::error!("Failed to get patch: {err}");
                CommandError::Internal
            })?;
            if let Some(patch) = patch {
                files.push(Self::process_patch(patch, &reviewd_files)?);
            }
        }

        Ok((change_id, files))
    }

    fn get_reviewd_files(
        change_id: Option<String>,
        db: &mut DB,
        github_node_id: &str,
        pr_number: u64,
    ) -> Result<HashSet<(PathBuf, PatchId)>> {
        let reviewed_files = db
            .reviewed_files()
            .github_node_id(github_node_id)
            .pr_number(pr_number as i64)
            .change_id(change_id.as_deref())
            .fetch()
            .map_err(|err| {
                log::error!("Failed to fetch reviewed files: {err}");
                CommandError::Internal
            })?;

        // Build lookup map (file_path, patch_id) -> reviewed
        let reviewed_files: HashSet<(PathBuf, PatchId)> = reviewed_files
            .into_iter()
            .map(|rf| (PathBuf::from(rf.file_path), rf.patch_id))
            .collect();
        Ok(reviewed_files)
    }

    fn process_line(line: git2::DiffLine) -> (DiffLine, u32, u32) {
        let line_type = Self::map_line_type(line.origin_value());
        let content = String::from_utf8_lossy(line.content()).to_string();

        // Count additions and deletions
        let (additions, deletions) = match line.origin_value() {
            Git2DiffLineType::Addition => (1, 0),
            Git2DiffLineType::Deletion => (0, 1),
            _ => (0, 0),
        };

        let diff_line = DiffLine {
            line_type,
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
            content,
        };

        (diff_line, additions, deletions)
    }

    fn process_hunk(patch: &git2::Patch, hunk_idx: usize) -> Result<(DiffHunk, u32, u32)> {
        let (hunk, hunk_lines_count) = patch.hunk(hunk_idx).map_err(|err| {
            log::error!("Failed to get hunk: {err}");
            CommandError::Internal
        })?;

        let mut lines = Vec::new();
        let mut hunk_additions = 0u32;
        let mut hunk_deletions = 0u32;

        // Process lines in this hunk
        for line_idx in 0..hunk_lines_count {
            let line = patch.line_in_hunk(hunk_idx, line_idx).map_err(|err| {
                log::error!("Failed to get line: {err}");
                CommandError::Internal
            })?;

            let (diff_line, add, del) = Self::process_line(line);
            hunk_additions += add;
            hunk_deletions += del;
            lines.push(diff_line);
        }

        let header = String::from_utf8_lossy(hunk.header()).to_string();

        let diff_hunk = DiffHunk {
            old_start: hunk.old_start(),
            old_lines: hunk.old_lines(),
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
            header,
            lines,
        };

        Ok((diff_hunk, hunk_additions, hunk_deletions))
    }

    fn process_patch(
        patch: git2::Patch,
        reviewd_files: &HashSet<(PathBuf, PatchId)>,
    ) -> Result<FileDiff> {
        let delta = patch.delta();
        let old_file = delta.old_file();
        let new_file = delta.new_file();

        let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
        let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

        let status = Self::map_delta_status(delta.status());
        let is_binary = old_file.is_binary() || new_file.is_binary();

        let mut additions = 0u32;
        let mut deletions = 0u32;
        let mut hunks = Vec::new();

        // Process all hunks
        for hunk_idx in 0..patch.num_hunks() {
            let (hunk, add, del) = Self::process_hunk(&patch, hunk_idx)?;
            additions += add;
            deletions += del;
            hunks.push(hunk);
        }

        // Compute patch-id (skip for binary files)
        let patch_id = if is_binary {
            None
        } else {
            Some(ReviewService::compute_file_patch_id(&patch)?)
        };

        let file_path = new_path.as_ref().or(old_path.as_ref()).map(PathBuf::from);
        let is_reviewed = match (file_path, patch_id.clone()) {
            (Some(file_path), Some(patch_id)) => reviewd_files.contains(&(file_path, patch_id)),
            _ => false,
        };

        Ok(FileDiff {
            old_path,
            new_path,
            status,
            additions,
            deletions,
            is_binary,
            hunks,
            patch_id,
            is_reviewed,
        })
    }

    fn map_delta_status(status: Delta) -> FileChangeStatus {
        match status {
            Delta::Added => FileChangeStatus::Added,
            Delta::Deleted => FileChangeStatus::Deleted,
            Delta::Modified => FileChangeStatus::Modified,
            Delta::Renamed => FileChangeStatus::Renamed,
            Delta::Copied => FileChangeStatus::Copied,
            Delta::Typechange => FileChangeStatus::Typechange,
            _ => FileChangeStatus::Modified, // Default for untracked, ignored, etc.
        }
    }

    fn map_line_type(line_type: Git2DiffLineType) -> DiffLineType {
        match line_type {
            Git2DiffLineType::Context | Git2DiffLineType::ContextEOFNL => DiffLineType::Context,
            Git2DiffLineType::Addition => DiffLineType::Addition,
            Git2DiffLineType::Deletion => DiffLineType::Deletion,
            Git2DiffLineType::AddEOFNL => DiffLineType::AddEofnl,
            Git2DiffLineType::DeleteEOFNL => DiffLineType::DelEofnl,
            _ => DiffLineType::Context, // Default for file headers, hunk headers, etc.
        }
    }
}
