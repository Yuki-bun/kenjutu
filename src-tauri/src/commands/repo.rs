use std::sync::Arc;
use tauri::{command, State};

use crate::errors::{CommandError, Result};
use crate::models::GhRepoId;
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
    use crate::db::LocalRepo;

    let mut db = app.get_connection()?;

    // Validate git repository
    if git2::Repository::open(&local_dir).is_err() {
        return Err(CommandError::bad_input(format!(
            "Directory {} is not a git repository",
            local_dir
        )));
    }

    let local_repo = LocalRepo {
        gh_id: repo_id,
        local_dir: Some(local_dir),
    };

    db.upsert_local_repo(local_repo).map_err(|err| {
        log::error!("DB error: {err}");
        CommandError::Internal
    })?;

    Ok(())
}
