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
    #[error("Failed to calculate base for commit with multiple parents:  commit_id={commit_id}")]
    BasesMergeConflict { commit_id: Oid },
    #[error("Invalid ChangeId: expected a 32-character string, got '{received}'")]
    InvalidChangeId { received: String },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ChangeId([u8; 32]);

impl std::fmt::Debug for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl From<[u8; 32]> for ChangeId {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl TryFrom<&str> for ChangeId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let bytes = value.as_bytes();
        if bytes.len() != 32 {
            return Err(Error::InvalidChangeId {
                received: value.to_string(),
            });
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Self(array))
    }
}
