use std::sync::Arc;
use tauri::{command, State};

use crate::errors::{CommandError, Result};
use crate::models::GhRepoId;
use crate::services::RepositoryService;
use crate::App;

#[command]
#[specta::specta]
pub async fn get_local_repo_path(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
) -> Result<Option<String>> {
    let mut db = app.get_connection()?;
    let local_repo = db.find_repository(&repo_id).map_err(|err| {
        log::error!("DB error: {err}");
        CommandError::Internal
    })?;
    Ok(local_repo.and_then(|r| r.local_dir))
}

#[command]
#[specta::specta]
pub async fn set_local_repo(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    local_dir: String,
) -> Result<()> {
    let github = app.github_service();
    let mut db = app.get_connection()?;
    RepositoryService::set_local_repository(&github, &mut db, &repo_id, &local_dir).await
}
