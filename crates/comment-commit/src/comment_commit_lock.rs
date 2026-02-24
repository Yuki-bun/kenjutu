use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

use git2::Repository;

use crate::{ChangeId, CommitId, Result};

/// A file-based exclusive lock for comment-commit writes.
///
/// Lock path: `.git/info/kenjutu/comment-lock/{change_id}/{commit_sha}`
///
/// Uses a separate lock path from marker-commit to avoid contention between
/// review state writes and comment writes.
#[derive(Debug)]
pub struct CommentCommitLock {
    path: PathBuf,
    _lock_file: File,
}

impl CommentCommitLock {
    pub fn new(repo: &Repository, change_id: ChangeId, sha: CommitId) -> Result<Self> {
        let path = Self::lock_path(repo, change_id, sha);
        fs::create_dir_all(path.parent().unwrap())?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        file.lock_exclusive()?;

        log::info!("acquired comment lock at {}", path.to_str().unwrap_or(""));
        Ok(Self {
            _lock_file: file,
            path,
        })
    }

    pub fn lock_path(repo: &Repository, change_id: ChangeId, sha: CommitId) -> PathBuf {
        repo.path()
            .join("info/kenjutu/comment-lock")
            .join(change_id.to_string())
            .join(sha.to_string())
    }
}

impl Drop for CommentCommitLock {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.path) {
            log::warn!("failed to delete comment lock file: {}", err);
        }
        log::info!(
            "released comment lock at {}",
            self.path.to_str().unwrap_or("")
        );
    }
}
