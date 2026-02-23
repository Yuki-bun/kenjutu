use super::git;
use kenjutu_types::CommitId;

pub use file_diff::{generate_partial_review_diffs, get_context_lines, PartialReviewDiffs};
pub use file_list::generate_file_list;

mod file_diff;
mod file_list;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File not found in tree: {0}")]
    FileNotFound(String),

    #[error("Git error: {0}")]
    Git(#[from] git::Error),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Marker commit error: {0}")]
    MarkerCommit(#[from] marker_commit::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Conflicted parents in merge commit: {0}")]
    MergeConflict(CommitId),
}
