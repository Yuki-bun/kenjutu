use std::{collections::HashSet, path::PathBuf};

use rusqlite::{
    types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef},
    ToSql,
};
use rusqlite_from_row::FromRow;

use crate::{
    db::{RepoDb, Result},
    models::{ChangeId, PatchId},
};

#[derive(Debug, Clone, FromRow)]
pub struct ReviewedFile {
    #[expect(unused)]
    pub change_id: ChangeId,
    pub file_path: String,
    pub patch_id: PatchId,
    #[expect(unused)]
    pub reviewed_at: String,
}

pub struct ReviewedFileRepository<'a> {
    db: &'a RepoDb,
}

impl<'a> ReviewedFileRepository<'a> {
    pub fn new(db: &'a RepoDb) -> Self {
        Self { db }
    }

    pub fn get_reviewed_files_set(
        &self,
        change_id: &ChangeId,
    ) -> Result<HashSet<(PathBuf, PatchId)>> {
        let sql = r#"SELECT change_id, file_path, patch_id, reviewed_at FROM reviewed_files WHERE change_id=?"#;
        let mut stmt = self.db.conn().prepare(sql)?;
        let set = stmt
            .query_map([change_id], ReviewedFile::try_from_row)?
            .map(|res| res.map(|file| (PathBuf::from(file.file_path), file.patch_id)))
            .collect::<rusqlite::Result<_>>()?;
        Ok(set)
    }

    pub fn mark_file_reviewed(
        &self,
        change_id: ChangeId,
        file_path: String,
        patch_id: PatchId,
    ) -> Result<()> {
        self.db.conn().execute(
            "INSERT OR IGNORE INTO reviewed_files
             (change_id, file_path, patch_id, reviewed_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(change_id, file_path)
             DO UPDATE SET patch_id = excluded.patch_id;",
            rusqlite::params![
                change_id,
                file_path,
                patch_id,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn mark_file_not_reviewed(&self, change_id: &ChangeId, file_path: &str) -> Result<()> {
        self.db.conn().execute(
            "DELETE FROM reviewed_files
             WHERE change_id = ?
               AND file_path = ?",
            rusqlite::params![change_id, file_path],
        )?;
        Ok(())
    }
}

impl FromSql for PatchId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(PatchId::from)
    }
}

impl ToSql for PatchId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for ChangeId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(ChangeId::from)
    }
}

impl ToSql for ChangeId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}
