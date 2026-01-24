use std::collections::HashSet;
use std::path::PathBuf;

use crate::db::{ReviewedFile, DB};
use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, PatchId};

pub struct ReviewRepository<'a> {
    db: &'a mut DB,
}

impl<'a> ReviewRepository<'a> {
    pub fn new(db: &'a mut DB) -> Self {
        Self { db }
    }

    pub fn get_reviewed_files_set(
        &mut self,
        change_id: Option<&ChangeId>,
    ) -> Result<HashSet<(PathBuf, PatchId)>> {
        let reviewed_files = match change_id {
            Some(cid) => self
                .db
                .reviewed_files()
                .change_id(cid.clone())
                .fetch()
                .map_err(|err| {
                    log::error!("Failed to fetch reviewed files: {err}");
                    CommandError::Internal
                })?,
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
        self.db.insert_reviewed_file(reviewed_file).map_err(|err| {
            log::error!("Failed to insert reviewed file: {err}");
            CommandError::Internal
        })
    }

    pub fn mark_file_not_reviewed(&mut self, change_id: &ChangeId, file_path: &str) -> Result<()> {
        self.db
            .delete_reviewed_file(change_id, file_path)
            .map_err(|err| {
                log::error!("Failed to delete reviewed file: {err}");
                CommandError::Internal
            })
    }
}
