use serde::Serialize;

use kenjutu_types::ChangeId;

/// A commit from jj log output (for frontend consumption)
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct JjCommit {
    pub change_id: ChangeId,
    pub commit_id: String,
    /// First line of the commit message
    pub summary: String,
    /// Rest of the commit message (body), empty if none
    pub description: String,
    pub author: String,
    pub email: String,
    pub timestamp: String,
    pub is_immutable: bool,
    pub is_working_copy: bool,
    /// Parent change_ids (for graph edges) - supports multiple parents for merges
    pub parents: Vec<ChangeId>,
}

/// Response for get_jj_log command
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct JjLogResult {
    pub commits: Vec<JjCommit>,
}

/// Status of jj availability
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct JjStatus {
    pub is_installed: bool,
    pub is_jj_repo: bool,
}
