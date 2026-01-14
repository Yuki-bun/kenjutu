use sqlx::{pool::PoolConnection, Sqlite};

pub mod models;
pub use models::{LocalRepo, ReviewedFile};

pub struct DB {
    conn: PoolConnection<Sqlite>,
}

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlx(err) => write!(f, "sqlx error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

type Result<T> = std::result::Result<T, Error>;

impl DB {
    pub fn new(conn: PoolConnection<Sqlite>) -> Self {
        Self { conn }
    }

    pub async fn find_local_repo(&mut self, github_node_id: &str) -> Result<Option<LocalRepo>> {
        self.find_repository(github_node_id).await
    }

    pub async fn find_repository(&mut self, github_node_id: &str) -> Result<Option<LocalRepo>> {
        sqlx::query_as!(
            LocalRepo,
            "SELECT github_node_id, local_dir, owner, name FROM repository WHERE github_node_id = ?",
            github_node_id
        )
        .fetch_optional(&mut *self.conn)
        .await
        .map_err(Error::from)
    }

    pub async fn find_repository_by_owner_name(
        &mut self,
        owner: &str,
        name: &str,
    ) -> Result<Option<LocalRepo>> {
        sqlx::query_as!(
            LocalRepo,
            "SELECT github_node_id, local_dir, owner, name FROM repository WHERE owner = ? AND name = ?",
            owner,
            name
        )
        .fetch_optional(&mut *self.conn)
        .await
        .map_err(Error::from)
    }

    pub async fn upsert_repository_cache(
        &mut self,
        github_node_id: &str,
        owner: &str,
        name: &str,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO repository(github_node_id, local_dir, owner, name)
             VALUES (?, NULL, ?, ?)
             ON CONFLICT (github_node_id)
             DO UPDATE SET
                 owner = excluded.owner,
                 name = excluded.name",
            github_node_id,
            owner,
            name,
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn upsert_local_repo(&mut self, local_repo: LocalRepo) -> Result<()> {
        sqlx::query!(
            "INSERT INTO repository(github_node_id, local_dir, owner, name)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (github_node_id)
             DO UPDATE SET
                 local_dir = excluded.local_dir",
            local_repo.github_node_id,
            local_repo.local_dir,
            local_repo.owner,
            local_repo.name,
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    // CRUD operations for reviewed files

    /// CREATE: Insert a reviewed file record
    pub async fn insert_reviewed_file(&mut self, reviewed_file: ReviewedFile) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO reviewed_files
             (github_node_id, pr_number, change_id, file_path, patch_id, reviewed_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            reviewed_file.github_node_id,
            reviewed_file.pr_number,
            reviewed_file.change_id,
            reviewed_file.file_path,
            reviewed_file.patch_id,
            reviewed_file.reviewed_at,
        )
        .execute(&mut *self.conn)
        .await?;
        Ok(())
    }

    /// READ: Returns builder for flexible filtering
    pub fn reviewed_files(&mut self) -> ReviewedFileQueryBuilder<'_> {
        ReviewedFileQueryBuilder::new(&mut self.conn)
    }

    /// DELETE: Remove a reviewed file record
    pub async fn delete_reviewed_file(
        &mut self,
        github_node_id: &str,
        pr_number: i64,
        change_id: Option<&str>,
        file_path: &str,
        patch_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM reviewed_files
             WHERE github_node_id = ?
               AND pr_number = ?
               AND change_id IS ?
               AND file_path = ?
               AND patch_id = ?",
            github_node_id,
            pr_number,
            change_id,
            file_path,
            patch_id,
        )
        .execute(&mut *self.conn)
        .await?;
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
    conn: &'a mut PoolConnection<Sqlite>,
    github_node_id: Option<String>,
    pr_number: Option<i64>,
    change_id: FilterValue<String>,
    file_path: Option<String>,
    patch_id: Option<String>,
}

impl<'a> ReviewedFileQueryBuilder<'a> {
    fn new(conn: &'a mut PoolConnection<Sqlite>) -> Self {
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
    pub fn patch_id(mut self, id: impl Into<String>) -> Self {
        self.patch_id = Some(id.into());
        self
    }

    pub async fn fetch(self) -> Result<Vec<ReviewedFile>> {
        use sqlx::QueryBuilder;

        let mut query = QueryBuilder::new("SELECT * FROM reviewed_files WHERE 1=1");

        if let Some(id) = &self.github_node_id {
            query.push(" AND github_node_id = ");
            query.push_bind(id);
        }
        if let Some(num) = self.pr_number {
            query.push(" AND pr_number = ");
            query.push_bind(num);
        }
        match &self.change_id {
            FilterValue::Unset => {}
            FilterValue::Value(val) => {
                query.push(" AND change_id = ");
                query.push_bind(val);
            }
            FilterValue::Null => {
                query.push(" AND change_id IS NULL");
            }
        }
        if let Some(path) = &self.file_path {
            query.push(" AND file_path = ");
            query.push_bind(path);
        }
        if let Some(id) = &self.patch_id {
            query.push(" AND patch_id = ");
            query.push_bind(id);
        }

        query
            .build_query_as::<ReviewedFile>()
            .fetch_all(&mut **self.conn)
            .await
            .map_err(Error::from)
    }
}
