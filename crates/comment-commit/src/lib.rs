mod comment_commit;
mod comment_commit_lock;
mod materialize;
pub(crate) mod model;
mod porting;
mod tree_builder_ext;

pub use comment_commit::{CommentCommit, comment_ref_name, enumerate_comment_refs};
pub use kenjutu_types::{ChangeId, CommitId};
pub use model::{AnchorContext, DiffSide, MaterializedComment, MaterializedReply, PortedComment};
pub use porting::{find_anchor_position, get_all_ported_comments};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Comment not found: {comment_id}")]
    CommentNotFound { comment_id: String },
    #[error("Invalid action: {message}")]
    InvalidAction { message: String },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
