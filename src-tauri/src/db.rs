use sqlx::{pool::PoolConnection, Sqlite};

pub struct DB {
    conn: PoolConnection<Sqlite>,
}

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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

    pub async fn find_local_repo(&mut self, github_id: u64) -> Result<Option<LocalRepo>> {
        let id = i64::try_from(github_id).expect("seems like github_id can get really big");
        sqlx::query_as!(
            LocalRepo,
            "SELECT github_id, local_dir FROM repository WHERE github_id = ?",
            id
        )
        .fetch_optional(&mut *self.conn)
        .await
        .map_err(Error::from)
    }

    pub async fn upsert_local_repo(&mut self, local_repo: LocalRepo) -> Result<()> {
        sqlx::query!(
            "
        INSERT INTO repository(github_id, local_dir)
        VALUES (?, ?)
        ON CONFLICT (github_id, local_dir)
        DO UPDATE SET local_dir = ?
        ",
            local_repo.github_id,
            local_repo.local_dir,
            local_repo.local_dir,
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct LocalRepo {
    pub github_id: i64,
    pub local_dir: String,
}
