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

// ── Commit graph types ──────────────────────────────────────────────

/// The complete graph layout computed from jj's log output
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CommitGraph {
    /// One entry per visual row, in display order (top to bottom).
    /// Includes both commit rows and elision markers.
    pub rows: Vec<GraphRow>,
    /// Maximum number of columns used in the graph
    pub max_columns: usize,
}

/// A row in the commit graph — either a real commit or an elision marker
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GraphRow {
    Commit(Box<CommitRow>),
    Elision(ElisionRow),
}

/// A commit node in the graph
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CommitRow {
    pub commit: JjCommit,
    /// Which column (0-indexed) this commit's node sits in
    pub column: usize,
    /// The index of this row in the graph (0 = topmost)
    pub row: usize,
    /// Edges from this commit to its parents (or to an elision row)
    pub edges: Vec<GraphEdge>,
    /// Columns where vertical pass-through lines exist at this row.
    /// These are edges from other branches passing through without
    /// connecting to this commit.
    pub passing_columns: Vec<usize>,
}

/// Terminal elision marker — represents one or more hidden/elided revisions.
/// Has no outgoing edges. Renderers should draw "~" or a dashed terminator.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ElisionRow {
    /// The index of this row in the graph
    pub row: usize,
    /// Which column the elision marker sits in
    pub column: usize,
    /// Columns where vertical pass-through lines exist at this row
    pub passing_columns: Vec<usize>,
}

/// An edge from a commit to a parent (or to an elision marker)
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// The column of the child node (the commit that owns this edge)
    pub from_column: usize,
    /// The row index of the target (parent commit or elision marker)
    pub to_row: usize,
    /// The column of the target
    pub to_column: usize,
    /// How this edge should be rendered
    pub edge_type: EdgeType,
}

/// Classifies how a graph edge should be rendered
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum EdgeType {
    /// Straight vertical line — same column, direct parent
    Straight,
    /// Edge crosses columns — child and parent are in different columns
    CrossColumn,
    /// Second+ parent of a merge commit (crosses columns)
    Merge,
    /// Edge to a terminal elision row (draw as dotted/dashed)
    Elided,
}
