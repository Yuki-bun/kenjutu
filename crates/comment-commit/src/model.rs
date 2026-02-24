use serde::{Deserialize, Serialize};

/// A single entry in the append-only action log.
/// Each action has a unique ID for deduplication during future syncing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ActionEntry {
    pub(crate) action_id: String,
    pub(crate) created_at: String,
    pub(crate) action: CommentAction,
}

/// The set of actions that can be appended to the comment log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum CommentAction {
    /// Create a new top-level inline comment on a diff.
    Create {
        comment_id: String,
        side: DiffSide,
        line: u32,
        start_line: Option<u32>,
        body: String,
        anchor: AnchorContext,
    },
    /// Reply to an existing top-level comment (flat threads only).
    Reply {
        comment_id: String,
        parent_comment_id: String,
        body: String,
    },
    /// Edit the body of a comment or reply.
    Edit { comment_id: String, body: String },
    /// Resolve a thread (targets the root comment only).
    Resolve { comment_id: String },
    /// Unresolve a previously resolved thread (targets the root comment only).
    Unresolve { comment_id: String },
}

/// Which side of the diff the comment is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum DiffSide {
    Old,
    New,
}

/// Context lines around the commented line(s) for anchor-based porting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AnchorContext {
    /// ~3 lines before the commented line(s).
    pub before: Vec<String>,
    /// The commented line(s) themselves.
    pub target: Vec<String>,
    /// ~3 lines after the commented line(s).
    pub after: Vec<String>,
}

/// A fully materialized comment thread, produced by replaying the action log.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MaterializedComment {
    pub id: String,
    pub side: DiffSide,
    pub line: u32,
    pub start_line: Option<u32>,
    pub body: String,
    pub anchor: AnchorContext,
    pub resolved: bool,
    pub created_at: String,
    pub updated_at: String,
    pub edit_count: u32,
    pub replies: Vec<MaterializedReply>,
}

/// A single reply within a comment thread.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MaterializedReply {
    pub id: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub edit_count: u32,
}

/// A materialized comment with ported line numbers for display on a different commit.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PortedComment {
    pub comment: MaterializedComment,
    pub ported_line: Option<u32>,
    pub ported_start_line: Option<u32>,
    /// Whether this comment was ported from a different commit SHA.
    pub is_ported: bool,
}
