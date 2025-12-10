use tauri::{command, State};

use crate::errors::Result;
use crate::models::{FullRepo, Repo};
use crate::services::RepositoryService;
use crate::state::AppState;

#[command]
#[specta::specta]
pub async fn get_repositories(app: State<'_, AppState>) -> Result<Vec<Repo>> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    RepositoryService::get_repositories(&github).await
}

#[command]
#[specta::specta]
pub async fn get_repo_by_id(
    app: State<'_, AppState>,
    owner: String,
    name: String,
) -> Result<FullRepo> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;
    RepositoryService::get_repository_details(&github, &mut db, &owner, &name).await
}

#[command]
#[specta::specta]
pub async fn set_local_repo(
    app: State<'_, AppState>,
    owner: String,
    name: String,
    local_dir: String,
) -> Result<()> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;
    RepositoryService::set_local_repository(&github, &mut db, &owner, &name, &local_dir).await
}
