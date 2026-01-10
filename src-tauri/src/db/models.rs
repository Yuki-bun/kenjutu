#[derive(sqlx::FromRow, Debug, Clone)]
pub struct LocalRepo {
    pub github_node_id: String,
    pub local_dir: String,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ReviewedFile {
    pub github_node_id: String,
    pub pr_number: i64,
    pub change_id: Option<String>,
    pub file_path: String,
    pub patch_id: String,
    pub reviewed_at: String,
}
