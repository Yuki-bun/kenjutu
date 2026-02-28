mod apply_hunk;
mod conflict;
mod marker_commit;
mod marker_commit_lock;
mod materialize_tree;
mod octopus_merge;
mod tree_builder_ext;

pub use apply_hunk::HunkId;
pub use kenjutu_types::{ChangeId, CommitId};
pub use marker_commit::MarkerCommit;
pub use materialize_tree::materialize_tree;

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
        marker_commit_id: CommitId,
    },
    #[error("Failed to calculate base for commit with multiple parents:  commit_id={commit_id}")]
    BasesMergeConflict { commit_id: CommitId },
    #[error("File not found: {path}, old_path: {old_path:?}")]
    FileNotFound {
        path: String,
        old_path: Option<String>,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
