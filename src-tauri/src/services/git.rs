use git2::{Commit, Repository};

use crate::models::ChangeId;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Invalid SHA: {0}")]
    InvalidSha(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
}

pub fn open_repository(local_dir: &str) -> Result<Repository> {
    Repository::open(local_dir).map_err(|_| Error::RepoNotFound(local_dir.to_string()))
}

pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
    commit
        .header_field_bytes("change-id")
        .ok()
        .and_then(|buf| buf.as_str().map(String::from).map(ChangeId::from))
}
