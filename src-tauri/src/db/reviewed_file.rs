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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> RepoDb {
        RepoDb::open_in_memory().unwrap()
    }

    #[test]
    fn get_reviewed_files_set_returns_empty_when_no_files() {
        let db = setup_db();
        let repo = ReviewedFileRepository::new(&db);
        let change_id = ChangeId::from("change-1".to_string());

        let result = repo.get_reviewed_files_set(&change_id).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn mark_file_reviewed_inserts_new_record() {
        let db = setup_db();
        let repo = ReviewedFileRepository::new(&db);
        let change_id = ChangeId::from("change-1".to_string());
        let patch_id = PatchId::from("patch-1".to_string());

        repo.mark_file_reviewed(change_id.clone(), "src/main.rs".to_string(), patch_id.clone())
            .unwrap();

        let result = repo.get_reviewed_files_set(&change_id).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains(&(PathBuf::from("src/main.rs"), patch_id)));
    }

    #[test]
    fn mark_file_reviewed_updates_patch_id_on_conflict() {
        let db = setup_db();
        let repo = ReviewedFileRepository::new(&db);
        let change_id = ChangeId::from("change-1".to_string());
        let patch_id_1 = PatchId::from("patch-1".to_string());
        let patch_id_2 = PatchId::from("patch-2".to_string());

        repo.mark_file_reviewed(change_id.clone(), "src/main.rs".to_string(), patch_id_1)
            .unwrap();
        repo.mark_file_reviewed(change_id.clone(), "src/main.rs".to_string(), patch_id_2.clone())
            .unwrap();

        let result = repo.get_reviewed_files_set(&change_id).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains(&(PathBuf::from("src/main.rs"), patch_id_2)));
    }

    #[test]
    fn mark_file_not_reviewed_removes_record() {
        let db = setup_db();
        let repo = ReviewedFileRepository::new(&db);
        let change_id = ChangeId::from("change-1".to_string());
        let patch_id = PatchId::from("patch-1".to_string());

        repo.mark_file_reviewed(change_id.clone(), "src/main.rs".to_string(), patch_id)
            .unwrap();
        repo.mark_file_not_reviewed(&change_id, "src/main.rs")
            .unwrap();

        let result = repo.get_reviewed_files_set(&change_id).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn get_reviewed_files_set_filters_by_change_id() {
        let db = setup_db();
        let repo = ReviewedFileRepository::new(&db);
        let change_id_1 = ChangeId::from("change-1".to_string());
        let change_id_2 = ChangeId::from("change-2".to_string());
        let patch_id = PatchId::from("patch-1".to_string());

        repo.mark_file_reviewed(change_id_1.clone(), "src/main.rs".to_string(), patch_id.clone())
            .unwrap();
        repo.mark_file_reviewed(change_id_2.clone(), "src/lib.rs".to_string(), patch_id.clone())
            .unwrap();

        let result_1 = repo.get_reviewed_files_set(&change_id_1).unwrap();
        let result_2 = repo.get_reviewed_files_set(&change_id_2).unwrap();

        assert_eq!(result_1.len(), 1);
        assert!(result_1.contains(&(PathBuf::from("src/main.rs"), patch_id.clone())));
        assert_eq!(result_2.len(), 1);
        assert!(result_2.contains(&(PathBuf::from("src/lib.rs"), patch_id)));
    }
}
