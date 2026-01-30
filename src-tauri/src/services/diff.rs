use git2::{Delta, DiffLineType as Git2DiffLineType, Oid};
use std::collections::HashSet;
use std::path::PathBuf;

use super::git;
use crate::db::{self, ReviewedFileRepository};
use crate::models::{
    ChangeId, DiffHunk, DiffLine, DiffLineType, FileChangeStatus, FileDiff, FileEntry,
    HighlightToken, PatchId, SingleFileDiff,
};
use crate::services::{GitService, HighlightService, HighlightedFile, ReviewService};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File not found in tree: {0}")]
    FileNotFound(String),

    #[error("Git error: {0}")]
    Git(#[from] git::Error),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Review error: {0}")]
    Db(#[from] db::Error),
}

pub struct DiffService;

impl DiffService {
    fn process_line(
        line: git2::DiffLine,
        old_highlighted: &HighlightedFile,
        new_highlighted: &HighlightedFile,
    ) -> (DiffLine, u32, u32) {
        let line_type = Self::map_line_type(line.origin_value());

        // Look up pre-highlighted tokens by line number
        let tokens = match line.origin_value() {
            Git2DiffLineType::Deletion => {
                // Deletion: use old file's line number
                line.old_lineno()
                    .and_then(|n| old_highlighted.get(n).cloned())
                    .unwrap_or_else(|| Self::plain_tokens_from_content(&line))
            }
            Git2DiffLineType::Addition => {
                // Addition: use new file's line number
                line.new_lineno()
                    .and_then(|n| new_highlighted.get(n).cloned())
                    .unwrap_or_else(|| Self::plain_tokens_from_content(&line))
            }
            _ => {
                // Context: use new file (both should be identical)
                line.new_lineno()
                    .and_then(|n| new_highlighted.get(n).cloned())
                    .unwrap_or_else(|| Self::plain_tokens_from_content(&line))
            }
        };

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
            tokens,
        };

        (diff_line, additions, deletions)
    }

    /// Fallback: create plain tokens from raw line content
    fn plain_tokens_from_content(line: &git2::DiffLine) -> Vec<HighlightToken> {
        let content = String::from_utf8_lossy(line.content()).to_string();
        vec![HighlightToken {
            content,
            color: None,
        }]
    }

    fn process_hunk(
        patch: &git2::Patch,
        hunk_idx: usize,
        old_highlighted: &HighlightedFile,
        new_highlighted: &HighlightedFile,
    ) -> Result<(DiffHunk, u32, u32)> {
        let (hunk, hunk_lines_count) = patch.hunk(hunk_idx)?;

        let mut lines = Vec::new();
        let mut hunk_additions = 0u32;
        let mut hunk_deletions = 0u32;

        // Process lines in this hunk
        for line_idx in 0..hunk_lines_count {
            let line = patch.line_in_hunk(hunk_idx, line_idx)?;

            let (diff_line, add, del) = Self::process_line(line, old_highlighted, new_highlighted);
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
        repo: &git2::Repository,
        patch: git2::Patch,
        reviewed_files: &HashSet<(PathBuf, PatchId)>,
        highlighter: &HighlightService,
    ) -> Result<FileDiff> {
        let delta = patch.delta();
        let old_file = delta.old_file();
        let new_file = delta.new_file();

        let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
        let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

        let status = Self::map_delta_status(delta.status());
        let is_binary = old_file.is_binary() || new_file.is_binary();

        // Fetch and highlight full file contents
        let old_highlighted = if !is_binary {
            GitService::get_blob_content(repo, old_file.id())
                .map(|content| {
                    highlighter.highlight_file(&content, old_path.as_deref().unwrap_or(""))
                })
                .unwrap_or_else(HighlightedFile::empty)
        } else {
            HighlightedFile::empty()
        };

        let new_highlighted = if !is_binary {
            GitService::get_blob_content(repo, new_file.id())
                .map(|content| {
                    highlighter.highlight_file(&content, new_path.as_deref().unwrap_or(""))
                })
                .unwrap_or_else(HighlightedFile::empty)
        } else {
            HighlightedFile::empty()
        };

        let mut additions = 0u32;
        let mut deletions = 0u32;
        let mut hunks = Vec::new();

        // Process all hunks
        for hunk_idx in 0..patch.num_hunks() {
            let (hunk, add, del) =
                Self::process_hunk(&patch, hunk_idx, &old_highlighted, &new_highlighted)?;
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
            (Some(file_path), Some(patch_id)) => reviewed_files.contains(&(file_path, patch_id)),
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

    /// Generate a lightweight file list without blob fetching or syntax highlighting.
    /// This is fast because it only iterates over diff deltas and counts lines from patches.
    pub fn generate_file_list(
        repository: &git2::Repository,
        commit_sha: &str,
        review_repo: &ReviewedFileRepository,
    ) -> Result<(Option<ChangeId>, Vec<FileEntry>)> {
        // Find commit
        let oid = Oid::from_str(commit_sha)
            .map_err(|_| git::Error::InvalidSha(commit_sha.to_string()))?;

        let commit = repository
            .find_commit(oid)
            .map_err(|_| git::Error::CommitNotFound(oid.to_string()))?;

        // Extract change_id from commit
        let change_id = GitService::get_change_id(&commit);

        // Get commit tree and parent tree
        let commit_tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            Some(parent.tree()?)
        } else {
            None
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts
            .context_lines(3)
            .interhunk_lines(0)
            .ignore_whitespace(false);

        // Enable rename detection
        let mut find_opts = git2::DiffFindOptions::new();
        find_opts.renames(true);

        let mut diff = repository.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_opts),
        )?;

        // Apply rename detection
        diff.find_similar(Some(&mut find_opts))?;

        let reviewed_files = change_id.as_ref().map_or(Ok(HashSet::new()), |change_id| {
            review_repo.get_reviewed_files_set(change_id)
        })?;

        // Process all file deltas to extract metadata only
        let mut files: Vec<FileEntry> = Vec::new();
        for (delta_idx, _) in diff.deltas().enumerate() {
            let patch = git2::Patch::from_diff(&diff, delta_idx)?;
            if let Some(patch) = patch {
                files.push(Self::process_patch_metadata(&patch, &reviewed_files)?);
            }
        }

        Ok((change_id, files))
    }

    /// Extract metadata from a patch without fetching blob contents or syntax highlighting.
    fn process_patch_metadata(
        patch: &git2::Patch,
        reviewed_files: &HashSet<(PathBuf, PatchId)>,
    ) -> Result<FileEntry> {
        let delta = patch.delta();
        let old_file = delta.old_file();
        let new_file = delta.new_file();

        let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
        let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

        let status = Self::map_delta_status(delta.status());
        let is_binary = old_file.is_binary() || new_file.is_binary();

        // Count additions/deletions by iterating hunk lines (without blob fetch)
        let (additions, deletions) = Self::count_changes(patch)?;

        // Compute patch-id (skip for binary files)
        let patch_id = if is_binary {
            None
        } else {
            Some(ReviewService::compute_file_patch_id(patch)?)
        };

        let file_path = new_path.as_ref().or(old_path.as_ref()).map(PathBuf::from);
        let is_reviewed = match (file_path, patch_id.clone()) {
            (Some(file_path), Some(patch_id)) => reviewed_files.contains(&(file_path, patch_id)),
            _ => false,
        };

        Ok(FileEntry {
            old_path,
            new_path,
            status,
            additions,
            deletions,
            is_binary,
            patch_id,
            is_reviewed,
        })
    }

    /// Count additions and deletions from patch hunks without fetching blob content.
    fn count_changes(patch: &git2::Patch) -> Result<(u32, u32)> {
        let mut additions = 0u32;
        let mut deletions = 0u32;

        for hunk_idx in 0..patch.num_hunks() {
            let (_, hunk_lines_count) = patch.hunk(hunk_idx)?;

            for line_idx in 0..hunk_lines_count {
                let line = patch.line_in_hunk(hunk_idx, line_idx)?;

                match line.origin_value() {
                    Git2DiffLineType::Addition => additions += 1,
                    Git2DiffLineType::Deletion => deletions += 1,
                    _ => {}
                }
            }
        }

        Ok((additions, deletions))
    }

    /// Generate a highlighted diff for a single file.
    /// Uses pathspec to limit git2's diff to just the requested file.
    /// For renamed files, pass the old_path to enable proper rename detection.
    pub fn generate_single_file_diff(
        repository: &git2::Repository,
        commit_sha: &str,
        file_path: &str,
        old_path: Option<&str>,
        review_repo: &ReviewedFileRepository,
    ) -> Result<SingleFileDiff> {
        // Find commit
        let oid = Oid::from_str(commit_sha)
            .map_err(|_| git::Error::InvalidSha(commit_sha.to_string()))?;

        let commit = repository
            .find_commit(oid)
            .map_err(|_| git::Error::CommitNotFound(oid.to_string()))?;

        // Extract change_id from commit
        let change_id = GitService::get_change_id(&commit);

        // Get commit tree and parent tree
        let commit_tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            Some(parent.tree()?)
        } else {
            None
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts
            .context_lines(3)
            .interhunk_lines(0)
            .ignore_whitespace(false)
            .pathspec(file_path);

        // Include old path for rename detection
        if let Some(old) = old_path {
            diff_opts.pathspec(old);
        }

        // Enable rename detection
        let mut find_opts = git2::DiffFindOptions::new();
        find_opts.renames(true);

        let mut diff = repository.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_opts),
        )?;

        // Apply rename detection
        diff.find_similar(Some(&mut find_opts))?;

        let reviewed_files = change_id.as_ref().map_or(Ok(HashSet::new()), |change_id| {
            review_repo.get_reviewed_files_set(change_id)
        })?;

        let highlighter = HighlightService::global();

        // Find the matching file delta
        // Try to match by new_path first, then old_path (for deletions)
        for (delta_idx, delta) in diff.deltas().enumerate() {
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());

            let matches =
                new_path.as_deref() == Some(file_path) || old_path.as_deref() == Some(file_path);

            if matches {
                let patch = git2::Patch::from_diff(&diff, delta_idx)?;

                if let Some(patch) = patch {
                    let file_diff =
                        Self::process_patch(repository, patch, &reviewed_files, highlighter)?;

                    return Ok(SingleFileDiff {
                        old_path: file_diff.old_path,
                        new_path: file_diff.new_path,
                        status: file_diff.status,
                        additions: file_diff.additions,
                        deletions: file_diff.deletions,
                        is_binary: file_diff.is_binary,
                        hunks: file_diff.hunks,
                        patch_id: file_diff.patch_id,
                        is_reviewed: file_diff.is_reviewed,
                    });
                }
            }
        }

        Err(Error::FileNotFound(file_path.to_string()))
    }
}
