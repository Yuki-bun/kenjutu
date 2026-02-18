mod conflict;
mod marker_commit;
mod marker_commit_lock;
mod tree_builder_ext;

pub use marker_commit::MarkerCommit;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeId(String);

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
