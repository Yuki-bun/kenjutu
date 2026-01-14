pub mod models;
pub use models::{LocalRepo, ReviewedFile};

use rusqlite::{Connection, OptionalExtension};
use rusqlite_from_row::FromRow;

use crate::models::PatchId;

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

    pub fn find_local_repo(&mut self, github_node_id: &str) -> Result<Option<LocalRepo>> {
        self.find_repository(github_node_id)
    }

    pub fn find_repository(&mut self, github_node_id: &str) -> Result<Option<LocalRepo>> {
        self.conn
            .query_row(
                "SELECT github_node_id, local_dir, owner, name FROM repository WHERE github_node_id = ?",
                [github_node_id],
                LocalRepo::try_from_row,
            )
            .optional()
            .map_err(Error::from)
    }

    pub fn find_repository_by_owner_name(
        &mut self,
        owner: &str,
        name: &str,
    ) -> Result<Option<LocalRepo>> {
        self.conn
            .query_row(
                "SELECT github_node_id, local_dir, owner, name FROM repository WHERE owner = ? AND name = ?",
                [owner, name],
                LocalRepo::try_from_row,
            )
            .optional()
            .map_err(Error::from)
    }

    pub fn upsert_repository_cache(
        &mut self,
        github_node_id: &str,
        owner: &str,
        name: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO repository(github_node_id, local_dir, owner, name)
             VALUES (?, NULL, ?, ?)
             ON CONFLICT (github_node_id)
             DO UPDATE SET
                 owner = excluded.owner,
                 name = excluded.name",
            [github_node_id, owner, name],
        )?;

        Ok(())
    }

    pub fn upsert_local_repo(&mut self, local_repo: LocalRepo) -> Result<()> {
        self.conn.execute(
            "INSERT INTO repository(github_node_id, local_dir, owner, name)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (github_node_id)
             DO UPDATE SET
                 local_dir = excluded.local_dir",
            rusqlite::params![
                local_repo.github_node_id,
                local_repo.local_dir,
                local_repo.owner,
                local_repo.name,
            ],
        )?;

        Ok(())
    }

    // CRUD operations for reviewed files

    /// CREATE: Insert a reviewed file record
    pub fn insert_reviewed_file(&mut self, reviewed_file: ReviewedFile) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reviewed_files
             (github_node_id, pr_number, change_id, file_path, patch_id, reviewed_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                reviewed_file.github_node_id,
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
        github_node_id: &str,
        pr_number: i64,
        change_id: Option<&str>,
        file_path: &str,
        patch_id: &PatchId,
    ) -> Result<()> {
        // Build the SQL based on whether change_id is None or Some
        match change_id {
            None => {
                self.conn.execute(
                    "DELETE FROM reviewed_files
                     WHERE github_node_id = ?
                       AND pr_number = ?
                       AND change_id IS NULL
                       AND file_path = ?
                       AND patch_id = ?",
                    rusqlite::params![github_node_id, pr_number, file_path, patch_id],
                )?;
            }
            Some(change_id_val) => {
                self.conn.execute(
                    "DELETE FROM reviewed_files
                     WHERE github_node_id = ?
                       AND pr_number = ?
                       AND change_id = ?
                       AND file_path = ?
                       AND patch_id = ?",
                    rusqlite::params![
                        github_node_id,
                        pr_number,
                        change_id_val,
                        file_path,
                        patch_id
                    ],
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
    github_node_id: Option<String>,
    pr_number: Option<i64>,
    change_id: FilterValue<String>,
    file_path: Option<String>,
    patch_id: Option<PatchId>,
}

impl<'a> ReviewedFileQueryBuilder<'a> {
    fn new(conn: &'a mut Connection) -> Self {
        Self {
            conn,
            github_node_id: None,
            pr_number: None,
            change_id: FilterValue::Unset,
            file_path: None,
            patch_id: None,
        }
    }

    pub fn github_node_id(mut self, id: impl Into<String>) -> Self {
        self.github_node_id = Some(id.into());
        self
    }

    pub fn pr_number(mut self, num: i64) -> Self {
        self.pr_number = Some(num);
        self
    }

    pub fn change_id(mut self, id: Option<impl Into<String>>) -> Self {
        self.change_id = match id {
            Some(val) => FilterValue::Value(val.into()),
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
        let mut sql = String::from("SELECT github_node_id, pr_number, change_id, file_path, patch_id, reviewed_at FROM reviewed_files WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(id) = &self.github_node_id {
            sql.push_str(" AND github_node_id = ?");
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
