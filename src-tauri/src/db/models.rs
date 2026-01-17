use rusqlite::{
    types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef},
    ToSql,
};
use rusqlite_from_row::FromRow;

use crate::models::{ChangeId, GhRepoId, PatchId};

#[derive(Debug, Clone, FromRow)]
pub struct LocalRepo {
    pub gh_id: GhRepoId,
    pub local_dir: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReviewedFile {
    pub gh_repo_id: GhRepoId,
    pub pr_number: i64,
    pub change_id: Option<ChangeId>,
    pub file_path: String,
    pub patch_id: PatchId,
    pub reviewed_at: String,
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

impl FromSql for GhRepoId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(GhRepoId::from)
    }
}

impl ToSql for GhRepoId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}
