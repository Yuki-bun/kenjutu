use std::collections::HashSet;
use std::path::PathBuf;

use crate::db::{self, RepoDb, ReviewedFile};
use crate::models::{ChangeId, PatchId};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Db(#[from] db::Error),
}

pub struct ReviewRepository<'a> {
    db: &'a mut RepoDb,
}

impl<'a> ReviewRepository<'a> {
    pub fn new(db: &'a mut RepoDb) -> Self {
        Self { db }
    }

    pub fn get_reviewed_files_set(
        &mut self,
        change_id: Option<&ChangeId>,
    ) -> Result<HashSet<(PathBuf, PatchId)>> {
        let reviewed_files = match change_id {
            Some(cid) => self.db.reviewed_files().change_id(cid.clone()).fetch()?,
            None => Vec::new(),
        };

        let reviewed_set: HashSet<(PathBuf, PatchId)> = reviewed_files
            .into_iter()
            .map(|rf| (PathBuf::from(rf.file_path), rf.patch_id))
            .collect();

        Ok(reviewed_set)
    }

    pub fn mark_file_reviewed(
        &mut self,
        change_id: ChangeId,
        file_path: String,
        patch_id: PatchId,
    ) -> Result<()> {
        let reviewed_file = ReviewedFile {
            change_id,
            file_path,
            patch_id,
            reviewed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.insert_reviewed_file(reviewed_file)?;
        Ok(())
    }

    pub fn mark_file_not_reviewed(&mut self, change_id: &ChangeId, file_path: &str) -> Result<()> {
        self.db.delete_reviewed_file(change_id, file_path)?;
        Ok(())
    }
}
