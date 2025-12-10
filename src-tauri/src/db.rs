use sqlx::{pool::PoolConnection, Sqlite};

pub mod models;
pub use models::LocalRepo;

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
        sqlx::query_as!(
            LocalRepo,
            "SELECT github_node_id, local_dir FROM repository WHERE github_node_id = ?",
            github_node_id
        )
        .fetch_optional(&mut *self.conn)
        .await
        .map_err(Error::from)
    }

    pub async fn upsert_local_repo(&mut self, local_repo: LocalRepo) -> Result<()> {
        sqlx::query!(
            "
        PRAGMA foreign_keys = ON;
        INSERT INTO repository(github_node_id, local_dir)
        VALUES (?, ?)
        ON CONFLICT (github_node_id)
        DO UPDATE SET local_dir = ?
        ",
            local_repo.github_node_id,
            local_repo.local_dir,
            local_repo.local_dir,
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }
}
