use git2::{DiffLineType as Git2DiffLineType, Patch};
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;
use serde::Serialize;
use std::path::Path;
use two_face::re_exports::syntect::parsing::SyntaxReference;

use super::{Error, Result};
use crate::models::{DiffHunk, DiffLine, DiffLineType, FileDiff, HighlightToken};
use crate::services::git;
use crate::services::highlight::{self, HighlightService};
use crate::services::word_diff::{compute_word_diff, Block, HunkLines, SideLine};

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

fn map_line_type(line_type: Git2DiffLineType) -> DiffLineType {
    match line_type {
        Git2DiffLineType::Context | Git2DiffLineType::ContextEOFNL => DiffLineType::Context,
        Git2DiffLineType::Addition => DiffLineType::Addition,
        Git2DiffLineType::Deletion => DiffLineType::Deletion,
        Git2DiffLineType::AddEOFNL => DiffLineType::AddEofnl,
        Git2DiffLineType::DeleteEOFNL => DiffLineType::DelEofnl,
        _ => DiffLineType::Context,
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
                let tokens = tokens
                    .into_iter()
                    .map(|t| HighlightToken {
                        content: t.content,
                        color: t.color,
                        changed: false,
                    })
                    .collect();
                let tokens = merge_same_color_tokens(tokens);
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
                let tokens = merge_same_color_tokens(tokens);
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
                let tokens = merge_same_color_tokens(tokens);
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

fn merge_same_color_tokens(tokens: Vec<HighlightToken>) -> Vec<HighlightToken> {
    let mut merged: Vec<HighlightToken> = Vec::new();

    for token in tokens {
        if let Some(last) = merged.last_mut() {
            if last.color == token.color && last.changed == token.changed {
                last.content.push_str(&token.content);
                continue;
            }
        }
        merged.push(token);
    }

    merged
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

fn diff_blobs(
    old_content: &[u8],
    old_path: Option<&Path>,
    new_content: &[u8],
    new_path: Option<&Path>,
) -> Result<Vec<DiffHunk>> {
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts
        .context_lines(3)
        .interhunk_lines(0)
        .ignore_whitespace(false);

    let patch = Patch::from_buffers(
        old_content,
        old_path,
        new_content,
        new_path,
        Some(&mut diff_opts),
    )?;

    process_patch(&patch)
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PartialReviewDiffs {
    /// diff(M→T) — remaining unreviewed changes
    pub remaining: FileDiff,
    /// diff(B→M) — already reviewed changes
    pub reviewed: FileDiff,
}

/// Generate two diffs for a partially reviewed file:
/// - remaining: diff(M→T) — what's left to review
/// - reviewed: diff(B→M) — what's already been reviewed
pub fn generate_partial_review_diffs(
    repository: &git2::Repository,
    sha: CommitId,
    change_id: ChangeId,
    file_path: &Path,
    old_path: Option<&Path>,
) -> Result<PartialReviewDiffs> {
    let marker = MarkerCommit::get(repository, change_id, sha)?;
    let base_tree = marker.base_tree();
    let marker_tree = marker.marker_tree();

    let commit = repository
        .find_commit(sha.oid())
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;
    let target_tree = commit.tree()?;

    let empty: &[u8] = b"";

    // Resolve blob from target tree (T)
    let target_blob = match target_tree.get_path(file_path) {
        Ok(entry) => Some(repository.find_blob(entry.id())?),
        Err(e) if e.code() == git2::ErrorCode::NotFound => None,
        Err(e) => return Err(Error::from(e)),
    };
    let target_content = target_blob.as_ref().map(|b| b.content()).unwrap_or(empty);

    // Resolve blob from marker tree (M)
    // For renamed files, M may have the file at old_path (not yet reviewed) or file_path (after review started)
    let marker_blob = match marker_tree.get_path(file_path) {
        Ok(entry) => Some(repository.find_blob(entry.id())?),
        Err(e) if e.code() == git2::ErrorCode::NotFound => {
            // Try old_path for renamed files
            if let Some(op) = old_path {
                match marker_tree.get_path(op) {
                    Ok(entry) => Some(repository.find_blob(entry.id())?),
                    Err(e2) if e2.code() == git2::ErrorCode::NotFound => None,
                    Err(e2) => return Err(Error::from(e2)),
                }
            } else {
                None
            }
        }
        Err(e) => return Err(Error::from(e)),
    };
    let marker_content = marker_blob.as_ref().map(|b| b.content()).unwrap_or(empty);

    // Resolve blob from base tree (B)
    let base_lookup = old_path.unwrap_or(file_path);
    let base_blob = match base_tree.get_path(base_lookup) {
        Ok(entry) => Some(repository.find_blob(entry.id())?),
        Err(e) if e.code() == git2::ErrorCode::NotFound => None,
        Err(e) => return Err(Error::from(e)),
    };
    let base_content = base_blob.as_ref().map(|b| b.content()).unwrap_or(empty);

    // Remaining: diff(M→T)
    let remaining_hunks = diff_blobs(marker_content, old_path, target_content, Some(file_path))?;
    let remaining_new_file_lines = target_blob
        .as_ref()
        .map(|blob| String::from_utf8_lossy(blob.content()).lines().count() as u32)
        .unwrap_or(0);

    // Reviewed: diff(B→M)
    let reviewed_hunks = diff_blobs(base_content, old_path, marker_content, Some(file_path))?;
    let reviewed_new_file_lines = marker_blob
        .as_ref()
        .map(|blob| String::from_utf8_lossy(blob.content()).lines().count() as u32)
        .unwrap_or(0);

    Ok(PartialReviewDiffs {
        remaining: FileDiff {
            hunks: remaining_hunks,
            new_file_lines: remaining_new_file_lines,
        },
        reviewed: FileDiff {
            hunks: reviewed_hunks,
            new_file_lines: reviewed_new_file_lines,
        },
    })
}

/// Fetch context lines from a file blob at a given commit with syntax highlighting.
/// `start_line` and `end_line` are 1-based inclusive line numbers in the new file.
/// `old_start_line` is the corresponding 1-based line number in the old file for the first returned line.
pub fn get_context_lines(
    repository: &git2::Repository,
    sha: CommitId,
    file_path: &str,
    start_line: u32,
    end_line: u32,
    old_start_line: u32,
) -> Result<Vec<DiffLine>> {
    let commit = repository
        .find_commit(sha.oid())
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;

    let commit_tree = commit.tree()?;

    let entry = commit_tree
        .get_path(Path::new(file_path))
        .map_err(|_| Error::FileNotFound(file_path.to_string()))?;
    let blob = repository.find_blob(entry.id())?;

    let content = match std::str::from_utf8(blob.content()) {
        Ok(s) => s.to_string(),
        Err(_) => {
            log::warn!("File {file_path} at commit {sha} contains non-UTF-8 content");
            String::from_utf8_lossy(blob.content()).to_string()
        }
    };
    let all_lines: Vec<&str> = content.lines().collect();

    let start_idx = (start_line as usize).saturating_sub(1);
    let end_idx = (end_line as usize).min(all_lines.len());

    if start_idx >= all_lines.len() || start_idx >= end_idx {
        return Ok(Vec::new());
    }

    // Set up syntax highlighting - feed all lines from start to build correct parse state
    let highlight_service = HighlightService::global();
    let syntax = highlight_service
        .detect_syntax(file_path)
        .unwrap_or_else(|| highlight_service.default_syntax());
    let mut state = highlight_service.parse_and_highlight(syntax);

    // Feed lines before the requested range to build up parse state
    for line in &all_lines[..start_idx] {
        let line_with_newline = format!("{line}\n");
        let _ = state.highlight_line(&line_with_newline);
    }

    // Highlight and collect the requested lines
    let mut lines = Vec::with_capacity(end_idx - start_idx);
    for (i, line) in all_lines[start_idx..end_idx].iter().enumerate() {
        let line_with_newline = format!("{line}\n");
        let tokens = state.highlight_line(&line_with_newline);
        let line_num = start_line + i as u32;
        let old_line_num = old_start_line + i as u32;

        lines.push(DiffLine {
            line_type: DiffLineType::Context,
            old_lineno: Some(old_line_num),
            new_lineno: Some(line_num),
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

    Ok(lines)
}
