pub mod models;
pub use models::ReviewedFile;

use rusqlite::Connection;
use rusqlite_from_row::FromRow;

use crate::models::{ChangeId, PatchId};

pub struct RepoDb {
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS reviewed_files (
    change_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    patch_id TEXT NOT NULL,
    reviewed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (change_id, file_path)
);
"#;

impl RepoDb {
    pub fn open(repository: &git2::Repository) -> Result<Self> {
        let db_path = repository.path().join("pr-manager.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch(INIT_SQL)?;
        Ok(Self { conn })
    }

    /// INSERT a reviewed file record
    pub fn insert_reviewed_file(&mut self, reviewed_file: ReviewedFile) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reviewed_files
             (change_id, file_path, patch_id, reviewed_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(change_id, file_path)
             DO UPDATE SET patch_id = excluded.patch_id;",
            rusqlite::params![
                reviewed_file.change_id,
                reviewed_file.file_path,
                reviewed_file.patch_id,
                reviewed_file.reviewed_at,
            ],
        )?;
        Ok(())
    }

    /// Returns builder for flexible filtering
    pub fn reviewed_files(&mut self) -> ReviewedFileQueryBuilder<'_> {
        ReviewedFileQueryBuilder::new(&mut self.conn)
    }

    /// DELETE a reviewed file record
    pub fn delete_reviewed_file(&mut self, change_id: &ChangeId, file_path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM reviewed_files
             WHERE change_id = ?
               AND file_path = ?",
            rusqlite::params![change_id, file_path],
        )?;
        Ok(())
    }
}

// Builder pattern for flexible queries
pub struct ReviewedFileQueryBuilder<'a> {
    conn: &'a mut Connection,
    change_id: Option<ChangeId>,
    file_path: Option<String>,
    patch_id: Option<PatchId>,
}

impl<'a> ReviewedFileQueryBuilder<'a> {
    fn new(conn: &'a mut Connection) -> Self {
        Self {
            conn,
            change_id: None,
            file_path: None,
            patch_id: None,
        }
    }

    pub fn change_id(mut self, id: ChangeId) -> Self {
        self.change_id = Some(id);
        self
    }

    #[allow(unused)]
    pub fn file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    #[allow(unused)]
    pub fn patch_id(mut self, id: PatchId) -> Self {
        self.patch_id = Some(id);
        self
    }

    pub fn fetch(self) -> Result<Vec<ReviewedFile>> {
        let mut sql = String::from(
            "SELECT change_id, file_path, patch_id, reviewed_at FROM reviewed_files WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(id) = &self.change_id {
            sql.push_str(" AND change_id = ?");
            params.push(Box::new(id.clone()));
        }
        if let Some(path) = &self.file_path {
            sql.push_str(" AND file_path = ?");
            params.push(Box::new(path.clone()));
        }
        if let Some(id) = &self.patch_id {
            sql.push_str(" AND patch_id = ?");
            params.push(Box::new(id.clone()));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|p| &**p as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt.query_map(&param_refs[..], ReviewedFile::try_from_row)?;

        rows.collect::<rusqlite::Result<Vec<ReviewedFile>>>()
            .map_err(Error::from)
    }
}
