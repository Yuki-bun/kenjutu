pub mod models;
pub use models::{LocalRepo, ReviewedFile};

use rusqlite::{Connection, OptionalExtension};
use rusqlite_from_row::FromRow;

use crate::models::{ChangeId, GhRepoId, PatchId};

pub struct DB {
    conn: Connection,
}

#[derive(Debug)]
pub enum Error {
    DB(rusqlite::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DB(err) => write!(f, "rusqlite error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Self::DB(value)
    }
}

type Result<T> = std::result::Result<T, Error>;

impl DB {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn find_local_repo(&mut self, repo_id: &GhRepoId) -> Result<Option<LocalRepo>> {
        self.find_repository(repo_id)
    }

    pub fn find_repository(&mut self, repo_id: &GhRepoId) -> Result<Option<LocalRepo>> {
        self.conn
            .query_row(
                "SELECT gh_id, local_dir, owner, name FROM repository WHERE gh_id = ?",
                [repo_id],
                LocalRepo::try_from_row,
            )
            .optional()
            .map_err(Error::from)
    }

    pub fn upsert_local_repo(&mut self, local_repo: LocalRepo) -> Result<()> {
        self.conn.execute(
            "INSERT INTO repository(gh_id, local_dir)
             VALUES (?, ?)
             ON CONFLICT (gh_id)
             DO UPDATE SET
                 local_dir = excluded.local_dir",
            rusqlite::params![local_repo.gh_id, local_repo.local_dir,],
        )?;

        Ok(())
    }

    // CRUD operations for reviewed files

    /// CREATE: Insert a reviewed file record
    pub fn insert_reviewed_file(&mut self, reviewed_file: ReviewedFile) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reviewed_files
             (gh_repo_id, pr_number, change_id, file_path, patch_id, reviewed_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                reviewed_file.gh_repo_id,
                reviewed_file.pr_number,
                reviewed_file.change_id,
                reviewed_file.file_path,
                reviewed_file.patch_id,
                reviewed_file.reviewed_at,
            ],
        )?;
        Ok(())
    }

    /// READ: Returns builder for flexible filtering
    pub fn reviewed_files(&mut self) -> ReviewedFileQueryBuilder<'_> {
        ReviewedFileQueryBuilder::new(&mut self.conn)
    }

    /// DELETE: Remove a reviewed file record
    pub fn delete_reviewed_file(
        &mut self,
        repo_id: &GhRepoId,
        pr_number: i64,
        change_id: Option<&ChangeId>,
        file_path: &str,
        patch_id: &PatchId,
    ) -> Result<()> {
        // Build the SQL based on whether change_id is None or Some
        match change_id {
            None => {
                self.conn.execute(
                    "DELETE FROM reviewed_files
                     WHERE gh_repo_id = ?
                       AND pr_number = ?
                       AND change_id IS NULL
                       AND file_path = ?
                       AND patch_id = ?",
                    rusqlite::params![repo_id, pr_number, file_path, patch_id],
                )?;
            }
            Some(change_id_val) => {
                self.conn.execute(
                    "DELETE FROM reviewed_files
                     WHERE gh_repo_id = ?
                       AND pr_number = ?
                       AND change_id = ?
                       AND file_path = ?
                       AND patch_id = ?",
                    rusqlite::params![repo_id, pr_number, change_id_val, file_path, patch_id],
                )?;
            }
        }
        Ok(())
    }
}

// Filter value enum for handling NULL vs value vs unset
#[derive(Debug, Clone, Default)]
pub enum FilterValue<T> {
    #[default]
    Unset, // Don't filter by this field
    Value(T), // Filter by field = value
    Null,     // Filter by field IS NULL
}

// Builder pattern for flexible queries
pub struct ReviewedFileQueryBuilder<'a> {
    conn: &'a mut Connection,
    gh_repo_id: Option<String>,
    pr_number: Option<i64>,
    change_id: FilterValue<ChangeId>,
    file_path: Option<String>,
    patch_id: Option<PatchId>,
}

impl<'a> ReviewedFileQueryBuilder<'a> {
    fn new(conn: &'a mut Connection) -> Self {
        Self {
            conn,
            gh_repo_id: None,
            pr_number: None,
            change_id: FilterValue::Unset,
            file_path: None,
            patch_id: None,
        }
    }

    pub fn gh_repo_id(mut self, id: impl Into<String>) -> Self {
        self.gh_repo_id = Some(id.into());
        self
    }

    pub fn pr_number(mut self, num: i64) -> Self {
        self.pr_number = Some(num);
        self
    }

    pub fn change_id(mut self, id: Option<ChangeId>) -> Self {
        self.change_id = match id {
            Some(val) => FilterValue::Value(val),
            None => FilterValue::Null,
        };
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
        let mut sql = String::from("SELECT gh_repo_id, pr_number, change_id, file_path, patch_id, reviewed_at FROM reviewed_files WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(id) = &self.gh_repo_id {
            sql.push_str(" AND gh_repo_id = ?");
            params.push(Box::new(id.clone()));
        }
        if let Some(num) = self.pr_number {
            sql.push_str(" AND pr_number = ?");
            params.push(Box::new(num));
        }
        match &self.change_id {
            FilterValue::Unset => {}
            FilterValue::Value(val) => {
                sql.push_str(" AND change_id = ?");
                params.push(Box::new(val.clone()));
            }
            FilterValue::Null => {
                sql.push_str(" AND change_id IS NULL");
            }
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
