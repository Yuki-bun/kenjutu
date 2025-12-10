#[derive(sqlx::FromRow, Debug, Clone)]
pub struct LocalRepo {
    pub github_node_id: String,
    pub local_dir: String,
}
