use rusqlite::{
    types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef},
    ToSql,
};
use rusqlite_from_row::FromRow;

use crate::models::{ChangeId, PatchId};

#[derive(Debug, Clone, FromRow)]
pub struct ReviewedFile {
    pub change_id: ChangeId,
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
