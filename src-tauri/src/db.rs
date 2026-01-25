mod reviewed_file;
use rusqlite::Connection;

pub use reviewed_file::ReviewedFileRepository;

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

    fn conn(&self) -> &Connection {
        &self.conn
    }
}
