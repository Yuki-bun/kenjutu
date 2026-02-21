use serde::{Deserialize, Serialize};

use kenjutu_types::{ChangeId, CommitId};

/// Identifies a hunk by its unified diff header coordinates.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct HunkId {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
}

impl From<HunkId> for marker_commit::HunkId {
    fn from(h: HunkId) -> Self {
        Self {
            old_start: h.old_start,
            old_lines: h.old_lines,
            new_start: h.new_start,
            new_lines: h.new_lines,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PRCommit {
    pub change_id: ChangeId,
    pub sha: String,
    pub summary: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Typechange,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub hunks: Vec<DiffHunk>,
    /// Total number of lines in the new file (0 for deletions)
    pub new_file_lines: u32,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub tokens: Vec<HighlightToken>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct HighlightToken {
    /// The text content of this token
    pub content: String,
    /// CSS hex color (e.g., "#cf222e"), None for default foreground
    pub color: Option<String>,
    /// True if this token is part of a character-level change (for inline diff highlighting)
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    AddEofnl,
    DelEofnl,
}

/// Lightweight file entry for file list (no content/hunks)
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: FileChangeStatus,
    pub additions: u32,
    pub deletions: u32,
    pub is_binary: bool,
    pub review_status: ReviewStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ReviewStatus {
    Reviewed,
    /// Some of the changes are reviewed
    #[allow(dead_code)]
    PartiallyReviewed,
    Unreviewed,
    /// The diff doesn't exist anymore (e.g,. changes were reviewed but the content of the file was
    /// reverted to the base version)
    #[allow(dead_code)]
    ReviewedReverted,
}

/// Response for get_commit_file_list command
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CommitFileList {
    pub commit_sha: CommitId,
    pub change_id: ChangeId,
    pub files: Vec<FileEntry>,
}
