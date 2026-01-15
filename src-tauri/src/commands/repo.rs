use std::sync::Arc;
use tauri::{command, State};

use crate::errors::Result;
use crate::models::{FullRepo, Repo};
use crate::services::{RepositoryCacheService, RepositoryService};
use crate::App;

#[command]
#[specta::specta]
pub async fn get_repositories(app: State<'_, Arc<App>>) -> Result<Vec<Repo>> {
    let github = app.github_service();
    RepositoryService::get_repositories(&github).await
}

#[command]
#[specta::specta]
pub async fn lookup_repository_node_id(
    app: State<'_, Arc<App>>,
    owner: String,
    name: String,
) -> Result<String> {
    let github = app.github_service();
    let mut db = app.get_connection()?;
    RepositoryCacheService::lookup_node_id_by_owner_name(&github, &mut db, &owner, &name).await
}

#[command]
#[specta::specta]
pub async fn get_repo_by_id(app: State<'_, Arc<App>>, node_id: String) -> Result<FullRepo> {
    let github = app.github_service();
    let mut db = app.get_connection()?;
    RepositoryService::get_repository(&github, &mut db, &node_id).await
}

#[command]
#[specta::specta]
pub async fn set_local_repo(
    app: State<'_, Arc<App>>,
    node_id: String,
    local_dir: String,
) -> Result<()> {
    let github = app.github_service();
    let mut db = app.get_connection()?;
    RepositoryService::set_local_repository(&github, &mut db, &node_id, &local_dir).await
}
