use git2::{Delta, DiffLineType as Git2DiffLineType};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use two_face::re_exports::syntect::parsing::SyntaxReference;

use super::git;
use super::highlight::{self, HighlightService};
use super::review;
use super::word_diff::{compute_word_diff, Block, HunkLines, SideLine};
use crate::db::{self, ReviewedFileRepository};
use crate::models::{
    ChangeId, DiffHunk, DiffLine, DiffLineType, FileChangeStatus, FileDiff, FileEntry,
    HighlightToken, PatchId,
};

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

#[derive(Debug)]
struct Hunk<'a> {
    patch: &'a git2::Patch<'a>,
    hunk_idx: usize,
    hunk_lines_count: usize,
    hunk: git2::DiffHunk<'a>,
}

impl<'a> Hunk<'a> {
    fn new(patch: &'a git2::Patch<'a>, hunk_idx: usize) -> Result<Self> {
        let (hunk, hunk_lines_count) = patch.hunk(hunk_idx)?;
        Ok(Hunk {
            patch,
            hunk_idx,
            hunk_lines_count,
            hunk,
        })
    }

    fn lines(&'a self) -> impl Iterator<Item = Result<git2::DiffLine<'a>>> {
        (0..self.hunk_lines_count).map(move |line_idx| {
            self.patch
                .line_in_hunk(self.hunk_idx, line_idx)
                .map_err(Error::from)
        })
    }
}

impl<'a> std::ops::Deref for Hunk<'a> {
    type Target = git2::DiffHunk<'a>;

    fn deref(&self) -> &Self::Target {
        &self.hunk
    }
}

impl HunkLines for Hunk<'_> {
    fn blocks(&self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();

        for line_res in self.lines() {
            let Ok(line) = line_res else { continue };
            let Ok(content) = std::str::from_utf8(line.content()) else {
                continue;
            };

            match line.origin_value() {
                Git2DiffLineType::Context | Git2DiffLineType::ContextEOFNL => {
                    if !old_lines.is_empty() || !new_lines.is_empty() {
                        blocks.push(Block {
                            old_lines: std::mem::take(&mut old_lines),
                            new_lines: std::mem::take(&mut new_lines),
                        });
                    }
                }
                Git2DiffLineType::Deletion => {
                    if let Some(lineno) = line.old_lineno() {
                        old_lines.push(SideLine {
                            lineno,
                            content: content.to_string(),
                        });
                    }
                }
                Git2DiffLineType::Addition => {
                    if let Some(lineno) = line.new_lineno() {
                        new_lines.push(SideLine {
                            lineno,
                            content: content.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        if !old_lines.is_empty() || !new_lines.is_empty() {
            blocks.push(Block {
                old_lines,
                new_lines,
            });
        }

        blocks
    }
}

fn process_hunk(hunk: &Hunk, syntax: &SyntaxReference) -> Result<DiffHunk> {
    let word_diff = compute_word_diff(hunk);

    let highlight_service = HighlightService::global();
    let mut old_state = highlight_service.parse_and_highlight(syntax);
    let mut new_state = highlight_service.parse_and_highlight(syntax);

    let mut lines = Vec::new();

    for line in hunk.lines() {
        let line = line?;
        let line_str = String::from_utf8_lossy(line.content()).to_string();
        match map_line_type(line.origin_value()) {
            DiffLineType::Context => {
                let _ = old_state.highlight_line(&line_str);
                let tokens = new_state.highlight_line(&line_str);
                lines.push(DiffLine {
                    line_type: DiffLineType::Context,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                    tokens: tokens
                        .into_iter()
                        .map(|t| HighlightToken {
                            content: t.content,
                            color: t.color,
                            changed: false,
                        })
                        .collect(),
                });
            }
            DiffLineType::Deletion => {
                let tokens = old_state.highlight_line(&line_str);
                let info = line.old_lineno().and_then(|n| word_diff.deletions.get(&n));
                let ranges = info.map(|(_paired, ranges)| ranges);
                let tokens = apply_change_ranges_to_tokens(tokens, ranges);
                let new_lineno = info.map(|(paired, _)| *paired);
                lines.push(DiffLine {
                    line_type: DiffLineType::Deletion,
                    old_lineno: line.old_lineno(),
                    new_lineno,
                    tokens,
                });
            }
            DiffLineType::Addition => {
                let tokens = new_state.highlight_line(&line_str);
                let info = line.new_lineno().and_then(|n| word_diff.insertions.get(&n));
                let ranges = info.map(|(_paired, ranges)| ranges);
                let tokens = apply_change_ranges_to_tokens(tokens, ranges);
                let old_lineno = info.map(|(paired, _)| *paired);
                lines.push(DiffLine {
                    line_type: DiffLineType::Addition,
                    old_lineno,
                    new_lineno: line.new_lineno(),
                    tokens,
                });
            }
            _ => {}
        }
    }

    let header = String::from_utf8_lossy(hunk.header()).to_string();

    Ok(DiffHunk {
        old_start: hunk.old_start(),
        old_lines: hunk.old_lines(),
        new_start: hunk.new_start(),
        new_lines: hunk.new_lines(),
        header,
        lines,
    })
}

fn process_patch(patch: &git2::Patch) -> Result<Vec<DiffHunk>> {
    let delta = patch.delta();
    let old_file = delta.old_file();
    let new_file = delta.new_file();

    let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
    let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

    let mut hunks = Vec::new();

    let highlight_service = HighlightService::global();
    let syntax = new_path
        .as_ref()
        .or(old_path.as_ref())
        .and_then(|path| highlight_service.detect_syntax(path))
        .unwrap_or_else(|| highlight_service.default_syntax());

    for hunk_idx in 0..patch.num_hunks() {
        let hunk = Hunk::new(patch, hunk_idx)?;
        let hunk = process_hunk(&hunk, syntax)?;
        hunks.push(hunk);
    }

    Ok(hunks)
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

/// For merge commits with exactly 2 parents, compute the auto-merged tree
/// via `merge_trees()`. Returns `Some(tree)` to use as the diff base instead
/// of parent(0)'s tree. Returns `None` for non-merge commits, octopus merges,
/// or when the auto-merge has conflicts (falls back to parent(0) diff).
fn compute_merge_base_tree<'repo>(
    repo: &'repo git2::Repository,
    commit: &git2::Commit,
) -> Result<Option<git2::Tree<'repo>>> {
    if commit.parent_count() != 2 {
        return Ok(None);
    }

    let parent0 = commit.parent(0)?;
    let parent1 = commit.parent(1)?;

    let ancestor_oid = match repo.merge_base(parent0.id(), parent1.id()) {
        Ok(oid) => oid,
        Err(_) => return Ok(None),
    };
    let ancestor = repo.find_commit(ancestor_oid)?;

    let mut index =
        repo.merge_trees(&ancestor.tree()?, &parent0.tree()?, &parent1.tree()?, None)?;

    if index.has_conflicts() {
        return Ok(None);
    }

    let tree_oid = index.write_tree_to(repo)?;
    Ok(Some(repo.find_tree(tree_oid)?))
}

/// Generate a lightweight file list without blob fetching or syntax highlighting.
/// This is fast because it only iterates over diff deltas and counts lines from patches.
pub fn generate_file_list(
    repository: &git2::Repository,
    sha: git2::Oid,
    review_repo: &ReviewedFileRepository,
) -> Result<(Option<ChangeId>, Vec<FileEntry>)> {
    let commit = repository
        .find_commit(sha)
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;

    // Extract change_id from commit
    let change_id = git::get_change_id(&commit);

    // Get commit tree and parent tree
    let commit_tree = commit.tree()?;

    // For merge commits, use auto-merged tree as base; otherwise use parent(0)
    let parent_tree = if commit.parent_count() > 0 {
        compute_merge_base_tree(repository, &commit)?
            .or_else(|| commit.parent(0).ok().and_then(|p| p.tree().ok()))
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
            files.push(process_patch_metadata(&patch, &reviewed_files)?);
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

    let status = map_delta_status(delta.status());
    let is_binary = old_file.is_binary() || new_file.is_binary();

    // Count additions/deletions by iterating hunk lines (without blob fetch)
    let (additions, deletions) = count_changes(patch)?;

    // Compute patch-id (skip for binary files)
    let patch_id = if is_binary {
        None
    } else {
        Some(review::compute_file_patch_id(patch)?)
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
    sha: git2::Oid,
    file_path: &str,
    old_path: Option<&str>,
) -> Result<FileDiff> {
    let commit = repository
        .find_commit(sha)
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;

    // Get commit tree and parent tree
    let commit_tree = commit.tree()?;

    // For merge commits, use auto-merged tree as base; otherwise use parent(0)
    let parent_tree = if commit.parent_count() > 0 {
        compute_merge_base_tree(repository, &commit)?
            .or_else(|| commit.parent(0).ok().and_then(|p| p.tree().ok()))
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

    // Find the matching file delta
    // Try to match by new_path first, then old_path (for deletions)
    for (delta_idx, delta) in diff.deltas().enumerate() {
        let delta_old_path = delta
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().to_string());
        let delta_new_path = delta
            .new_file()
            .path()
            .map(|p| p.to_string_lossy().to_string());

        let matches = delta_new_path.as_deref() == Some(file_path)
            || delta_old_path.as_deref() == Some(file_path);

        if matches {
            let patch = git2::Patch::from_diff(&diff, delta_idx)?;

            if let Some(patch) = patch {
                let hunks = process_patch(&patch)?;

                // Count lines in the new file
                let new_file_lines = commit_tree
                    .get_path(Path::new(file_path))
                    .ok()
                    .and_then(|entry| repository.find_blob(entry.id()).ok())
                    .map(|blob| String::from_utf8_lossy(blob.content()).lines().count() as u32)
                    .unwrap_or(0);

                return Ok(FileDiff {
                    hunks,
                    new_file_lines,
                });
            }
        }
    }

    Err(Error::FileNotFound(file_path.to_string()))
}

fn apply_change_ranges_to_tokens(
    tokens: Vec<highlight::Token>,
    change_ranges: Option<&Vec<(usize, usize)>>,
) -> Vec<HighlightToken> {
    let Some(ranges) = change_ranges.filter(|range| !range.is_empty()) else {
        return tokens
            .into_iter()
            .map(|t| HighlightToken {
                changed: false,
                content: t.content,
                color: t.color,
            })
            .collect();
    };

    let mut result = Vec::with_capacity(tokens.len());
    let mut pos = 0usize;

    for token in tokens {
        let token_start = pos;
        let token_end = pos + token.content.len();
        let mut current_pos = token_start;

        while current_pos < token_end {
            let next_boundary = find_next_boundary(current_pos, token_end, ranges);
            let is_changed = is_in_change_range(current_pos, ranges);

            let slice_start = current_pos - token_start;
            let slice_end = next_boundary - token_start;

            if slice_end > slice_start {
                result.push(HighlightToken {
                    content: token.content[slice_start..slice_end].to_string(),
                    color: token.color.clone(),
                    changed: is_changed,
                });
            }

            current_pos = next_boundary;
        }

        pos = token_end;
    }

    result
}

fn is_in_change_range(pos: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| pos >= *start && pos < *end)
}

fn find_next_boundary(current_pos: usize, token_end: usize, ranges: &[(usize, usize)]) -> usize {
    let mut next = token_end;

    for (start, end) in ranges {
        if *start > current_pos && *start < next {
            next = *start;
        }
        if current_pos >= *start && current_pos < *end && *end < next {
            next = *end;
        }
    }

    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ReviewedFileRepository;
    use crate::test_utils::TestRepo;

    fn make_review_repo() -> db::RepoDb {
        db::RepoDb::open_in_memory().unwrap()
    }

    // ── generate_file_list tests ────────────────────────────────────────

    #[test]
    fn file_list_added_file() {
        let t = TestRepo::new();
        t.write_file("hello.rs", "fn main() {}\n");
        let sha = t.commit("add hello.rs");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (change_id, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert!(change_id.is_none());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Added);
        assert_eq!(files[0].new_path.as_deref(), Some("hello.rs"));
        assert!(files[0].additions > 0);
        assert_eq!(files[0].deletions, 0);
        assert!(!files[0].is_binary);
        assert!(files[0].patch_id.is_some());
        assert!(!files[0].is_reviewed);
    }

    #[test]
    fn file_list_modified_file() {
        let t = TestRepo::new();
        t.write_file("lib.rs", "fn old() {}\n");
        t.commit("initial");
        t.write_file("lib.rs", "fn new() {}\nfn extra() {}\n");
        let sha = t.commit("modify");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Modified);
        assert_eq!(files[0].old_path.as_deref(), Some("lib.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("lib.rs"));
        assert!(files[0].additions > 0);
        assert!(files[0].deletions > 0);
    }

    #[test]
    fn file_list_deleted_file() {
        let t = TestRepo::new();
        t.write_file("temp.rs", "fn gone() {}\n");
        t.commit("initial");
        t.delete_file("temp.rs");
        let sha = t.commit("delete");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Deleted);
        assert_eq!(files[0].old_path.as_deref(), Some("temp.rs"));
        assert_eq!(files[0].additions, 0);
        assert!(files[0].deletions > 0);
    }

    #[test]
    fn file_list_renamed_file() {
        // Use 10+ lines so git2 rename detection has enough content to match
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n\
                        line 6\nline 7\nline 8\nline 9\nline 10\n\
                        line 11\nline 12\n";
        let t = TestRepo::new();

        t.write_file("old_name.rs", content);
        t.commit("initial");
        t.rename_file("old_name.rs", "new_name.rs");
        let sha = t.commit("rename");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Renamed);
        assert_eq!(files[0].old_path.as_deref(), Some("old_name.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new_name.rs"));
    }

    #[test]
    fn file_list_multiple_files() {
        let t = TestRepo::new();
        t.write_file("a.rs", "a\n");
        t.write_file("b.rs", "b\n");
        t.write_file("c.rs", "c\n");
        t.commit("initial");

        t.write_file("a.rs", "aa\n");
        t.write_file("b.rs", "bb\n");
        t.write_file("c.rs", "cc\n");
        let sha = t.commit("modify all");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert_eq!(files.len(), 3);
        let mut paths: Vec<_> = files.iter().filter_map(|f| f.new_path.as_deref()).collect();
        paths.sort();
        assert_eq!(paths, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn file_list_addition_deletion_counts() {
        let t = TestRepo::new();
        t.write_file("count.txt", "line1\nline2\nline3\nline4\nline5\n");
        t.commit("initial");

        // Change 2 lines (line1, line2) and add 1 new line → 3 additions, 2 deletions
        t.write_file("count.txt", "LINE1\nLINE2\nline3\nline4\nline5\nnew line\n");
        let sha = t.commit("modify");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, sha, &review_repo).unwrap();

        assert_eq!(files[0].additions, 3);
        assert_eq!(files[0].deletions, 2);
    }

    // ── generate_single_file_diff tests ─────────────────────────────────

    #[test]
    fn single_diff_modification() {
        let t = TestRepo::new();
        t.write_file("main.rs", "fn main() {\n    println!(\"hello\");\n}\n");
        t.commit("initial");

        t.write_file("main.rs", "fn main() {\n    println!(\"world\");\n}\n");
        let sha = t.commit("modify");

        let FileDiff { hunks, .. } =
            generate_single_file_diff(&t.repo, sha, "main.rs", None).unwrap();

        assert!(!hunks.is_empty());

        let lines = &hunks.iter().flat_map(|h| &h.lines).collect::<Vec<_>>();
        let deletions: Vec<_> = lines
            .iter()
            .filter(|l| l.line_type == DiffLineType::Deletion)
            .collect();
        let additions: Vec<_> = lines
            .iter()
            .filter(|l| l.line_type == DiffLineType::Addition)
            .collect();

        assert_eq!(deletions.len(), 1);
        assert_eq!(additions.len(), 1);

        // Token content concatenated should match the source lines
        let del_content: String = deletions[0]
            .tokens
            .iter()
            .map(|t| t.content.as_str())
            .collect();
        let add_content: String = additions[0]
            .tokens
            .iter()
            .map(|t| t.content.as_str())
            .collect();
        assert!(del_content.contains("hello"));
        assert!(add_content.contains("world"));

        // Deletion line has old_lineno set; matched deletions also have new_lineno
        assert!(deletions[0].old_lineno.is_some());
        assert!(
            deletions[0].new_lineno.is_some(),
            "matched deletion should have paired new_lineno"
        );
        // Addition line has new_lineno set; matched additions also have old_lineno
        assert!(additions[0].new_lineno.is_some());
        assert!(
            additions[0].old_lineno.is_some(),
            "matched addition should have paired old_lineno"
        );
    }

    #[test]
    fn single_diff_added_file() {
        let t = TestRepo::new();
        t.write_file("new.txt", "line one\nline two\n");
        let sha = t.commit("initial");

        let FileDiff { hunks, .. } =
            generate_single_file_diff(&t.repo, sha, "new.txt", None).unwrap();

        let lines: Vec<_> = hunks.iter().flat_map(|h| &h.lines).collect();
        assert_eq!(lines.len(), 2);

        for line in &lines {
            assert_eq!(line.line_type, DiffLineType::Addition);
            assert!(line.old_lineno.is_none());
            assert!(line.new_lineno.is_some());
        }

        assert_eq!(lines[0].new_lineno, Some(1));
        assert_eq!(lines[1].new_lineno, Some(2));
    }

    #[test]
    fn single_diff_deleted_file() {
        let t = TestRepo::new();
        t.write_file("doomed.txt", "aaa\nbbb\nccc\n");
        t.commit("initial");

        t.delete_file("doomed.txt");
        let sha = t.commit("delete");

        let FileDiff { hunks, .. } =
            generate_single_file_diff(&t.repo, sha, "doomed.txt", None).unwrap();

        let lines: Vec<_> = hunks.iter().flat_map(|h| &h.lines).collect();
        assert_eq!(lines.len(), 3);

        for line in &lines {
            assert_eq!(line.line_type, DiffLineType::Deletion);
            assert!(line.new_lineno.is_none());
            assert!(line.old_lineno.is_some());
        }

        assert_eq!(lines[0].old_lineno, Some(1));
        assert_eq!(lines[1].old_lineno, Some(2));
        assert_eq!(lines[2].old_lineno, Some(3));
    }

    #[test]
    fn single_diff_multiple_hunks() {
        // 30-line file, change lines 3 and 27 — far enough apart for separate hunks
        let original: String = (1..=30).map(|i| format!("line {i}\n")).collect();
        let mut modified_lines: Vec<String> = (1..=30).map(|i| format!("line {i}\n")).collect();
        modified_lines[2] = "CHANGED line 3\n".to_string();
        modified_lines[26] = "CHANGED line 27\n".to_string();
        let modified: String = modified_lines.concat();

        let t = TestRepo::new();
        t.write_file("big.txt", &original);
        t.commit("initial");

        t.write_file("big.txt", &modified);
        let sha = t.commit("modify");

        let FileDiff { hunks, .. } =
            generate_single_file_diff(&t.repo, sha, "big.txt", None).unwrap();

        assert_eq!(hunks.len(), 2);

        // Each hunk should have exactly 1 deletion and 1 addition
        for hunk in &hunks {
            let dels = hunk
                .lines
                .iter()
                .filter(|l| l.line_type == DiffLineType::Deletion)
                .count();
            let adds = hunk
                .lines
                .iter()
                .filter(|l| l.line_type == DiffLineType::Addition)
                .count();
            assert_eq!(dels, 1);
            assert_eq!(adds, 1);
        }
    }

    #[test]
    fn single_diff_renamed_file() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n\
                        line 6\nline 7\nline 8\nline 9\nline 10\n\
                        line 11\nline 12\n";
        // Modify one line so there's a diff to verify
        let modified = "line 1\nline 2\nCHANGED\nline 4\nline 5\n\
                         line 6\nline 7\nline 8\nline 9\nline 10\n\
                         line 11\nline 12\n";

        let t = TestRepo::new();

        t.write_file("old.rs", content);
        t.commit("initial");

        t.write_file("new.rs", modified);
        t.delete_file("old.rs");
        let sha = t.commit("rename");

        let FileDiff { hunks, .. } =
            generate_single_file_diff(&t.repo, sha, "new.rs", Some("old.rs")).unwrap();

        assert!(!hunks.is_empty());

        // Should contain the modification, not a full file add/delete
        let lines: Vec<_> = hunks.iter().flat_map(|h| &h.lines).collect();
        let has_context = lines.iter().any(|l| l.line_type == DiffLineType::Context);
        assert!(has_context, "renamed file diff should have context lines");
    }

    #[test]
    fn single_diff_file_not_found() {
        let t = TestRepo::new();
        t.write_file("exists.rs", "fn x() {}\n");
        let sha = t.commit("initial");

        let result = generate_single_file_diff(&t.repo, sha, "nope.rs", None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::FileNotFound(ref p) if p == "nope.rs"),
            "expected FileNotFound, got: {err:?}"
        );
    }

    // ── merge commit tests ──────────────────────────────────────────────

    #[test]
    fn pure_merge_has_empty_file_list() {
        let t = TestRepo::new();
        // Commit A: file_a.txt
        t.write_file("file_a.txt", "hello\n");
        let sha_a = t.commit("add file_a");
        // Commit B (child of A): adds file_b.txt
        t.write_file("file_b.txt", "world\n");
        let sha_b = t.commit("add file_b");

        // Pure merge: parents=[A, B], tree identical to B (both files, same blobs)
        t.write_file("file_a.txt", "hello\n");
        t.write_file("file_b.txt", "world\n");
        let merge_sha = t.commit_with_parents(&[sha_a, sha_b], "merge");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, merge_sha, &review_repo).unwrap();

        assert!(
            files.is_empty(),
            "pure merge should have empty file list, got {} files: {:?}",
            files.len(),
            files.iter().map(|f| &f.new_path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn merge_with_conflict_resolution_shows_resolved_file() {
        let t = TestRepo::new();
        // Commit A: base
        t.write_file("file.txt", "base\n");
        let sha_a = t.commit("base");
        // Commit B (child of A): branch change
        t.write_file("file.txt", "from-branch\n");
        let sha_b = t.commit("branch");
        // Commit C (child of A): main change — need to reset HEAD to A first
        // We'll use commit_merge to create C with parent A
        t.write_file("file.txt", "from-main\n");
        let sha_c = t.commit_with_parents(&[sha_a], "main change");

        // Merge M: parents=[C, B], tree has manually resolved content
        t.write_file("file.txt", "resolved\n");
        let merge_sha = t.commit_with_parents(&[sha_c, sha_b], "merge with resolution");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, merge_sha, &review_repo).unwrap();

        assert_eq!(
            files.len(),
            1,
            "merge with conflict resolution should show 1 file"
        );
    }

    #[test]
    fn pure_merge_single_file_diff_returns_empty() {
        let t = TestRepo::new();
        t.write_file("file_a.txt", "hello\n");
        let sha_a = t.commit("add file_a");
        t.write_file("file_b.txt", "world\n");
        let sha_b = t.commit("add file_b");

        t.write_file("file_a.txt", "hello\n");
        t.write_file("file_b.txt", "world\n");
        let merge_sha = t.commit_with_parents(&[sha_a, sha_b], "merge");

        // file_b.txt exists in the merge tree but is inherited from parent B
        // so the diff should be empty (not FileNotFound, just empty hunks)
        let result = generate_single_file_diff(&t.repo, merge_sha, "file_b.txt", None);

        match result {
            Ok(file_diff) => assert!(
                file_diff.hunks.is_empty(),
                "pure merge should return empty hunks for inherited file"
            ),
            Err(Error::FileNotFound(_)) => {
                // Also acceptable — the file was filtered out entirely
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn merge_both_parents_modify_same_file_no_conflict() {
        // Both parents modify the same file in non-conflicting regions.
        // The auto-merged result differs from ALL parents, but it's still
        // a pure merge — no manual intervention needed.
        let original: String = (1..=20).map(|i| format!("line {i}\n")).collect();

        let mut branch_lines: Vec<String> = (1..=20).map(|i| format!("line {i}\n")).collect();
        branch_lines[2] = "CHANGED-BY-BRANCH line 3\n".to_string();
        let branch_content: String = branch_lines.concat();

        let mut main_lines: Vec<String> = (1..=20).map(|i| format!("line {i}\n")).collect();
        main_lines[17] = "CHANGED-BY-MAIN line 18\n".to_string();
        let main_content: String = main_lines.concat();

        // Auto-merged: both changes applied
        let mut merged_lines: Vec<String> = (1..=20).map(|i| format!("line {i}\n")).collect();
        merged_lines[2] = "CHANGED-BY-BRANCH line 3\n".to_string();
        merged_lines[17] = "CHANGED-BY-MAIN line 18\n".to_string();
        let merged_content: String = merged_lines.concat();

        let t = TestRepo::new();
        // Ancestor commit A
        t.write_file("file.txt", &original);
        let sha_a = t.commit("ancestor");
        // Branch commit B (child of A)
        t.write_file("file.txt", &branch_content);
        let sha_b = t.commit("branch change");
        // Main commit C (child of A, via commit_merge with single parent)
        t.write_file("file.txt", &main_content);
        let sha_c = t.commit_with_parents(&[sha_a], "main change");

        // Merge M: parents=[C, B], tree = auto-merged (both changes)
        t.write_file("file.txt", &merged_content);
        let merge_sha = t.commit_with_parents(&[sha_c, sha_b], "merge");

        let db = make_review_repo();
        let review_repo = ReviewedFileRepository::new(&db);
        let (_, files) = generate_file_list(&t.repo, merge_sha, &review_repo).unwrap();

        assert!(
            files.is_empty(),
            "auto-merge with no conflicts should have empty file list, got {} files",
            files.len(),
        );
    }
}
