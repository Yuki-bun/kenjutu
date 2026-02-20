mod apply_hunk;
mod conflict;
mod marker_commit;
mod marker_commit_lock;
mod octopus_merge;
mod tree_builder_ext;

pub use apply_hunk::HunkId;
use git2::Oid;
pub use marker_commit::MarkerCommit;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error(
        "Marker commit has non-1 parent: change_id={change_id}, parent_count={parent_count}, marker_commit_id={marker_commit_id}"
    )]
    MarkerCommitNonOneParent {
        change_id: ChangeId,
        parent_count: usize,
        marker_commit_id: Oid,
    },
    #[error(
        "Failed to calculate base for commit with multiple parents: change_id={change_id}, commit_id={commit_id}"
    )]
    BasesMergeConflict { change_id: ChangeId, commit_id: Oid },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeId(String);

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ChangeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ChangeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
