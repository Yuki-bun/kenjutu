use serde::Serialize;
use specta::Type;

use super::{ChangeId, PatchId};

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PRCommit {
    pub change_id: Option<ChangeId>,
    pub sha: String,
    pub summary: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Typechange,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub tokens: Vec<HighlightToken>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HighlightToken {
    /// The text content of this token
    pub content: String,
    /// CSS hex color (e.g., "#cf222e"), None for default foreground
    pub color: Option<String>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    AddEofnl,
    DelEofnl,
}

/// Lightweight file entry for file list (no content/hunks)
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: FileChangeStatus,
    pub additions: u32,
    pub deletions: u32,
    pub is_binary: bool,
    pub patch_id: Option<PatchId>,
    pub is_reviewed: bool,
}

/// Response for get_commit_file_list command
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileList {
    pub commit_sha: String,
    pub change_id: Option<ChangeId>,
    pub files: Vec<FileEntry>,
}
