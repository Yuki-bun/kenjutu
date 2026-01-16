use serde::Serialize;
use specta::Type;

use super::{ChangeId, PatchId, User};

#[derive(Serialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub github_url: Option<String>,
    pub id: u64,
    pub title: Option<String>,
    pub author: Option<User>,
    pub number: u64,
}

impl From<octocrab::models::pulls::PullRequest> for PullRequest {
    fn from(value: octocrab::models::pulls::PullRequest) -> Self {
        Self {
            github_url: value.html_url.map(|url| url.into()),
            id: value.id.0,
            title: value.title,
            author: value.user.map(|owner| User::from(*owner)),
            number: value.number,
        }
    }
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetPullResponse {
    pub title: String,
    pub body: String,
    pub base_branch: String,
    pub head_branch: String,
    pub commits: Vec<PRCommit>,
    pub mergable: bool,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PRCommit {
    pub change_id: Option<ChangeId>,
    pub sha: String,
    pub summary: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MergePullResponse {
    pub sha: String,
    pub merged: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitDiff {
    pub commit_sha: String,
    pub change_id: Option<ChangeId>,
    pub files: Vec<FileDiff>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: FileChangeStatus,
    pub additions: u32,
    pub deletions: u32,
    pub is_binary: bool,
    pub hunks: Vec<DiffHunk>,
    pub patch_id: Option<PatchId>,
    pub is_reviewed: bool,
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
    pub content: String,
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
