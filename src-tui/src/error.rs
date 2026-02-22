use kenjutu_core::services::{diff, git, jj};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git::Error),

    #[error("Diff error: {0}")]
    Diff(#[from] diff::Error),

    #[error("Jj error: {0}")]
    Jj(#[from] jj::Error),

    #[error("Marker commit error: {0}")]
    MarkerCommit(#[from] marker_commit::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
