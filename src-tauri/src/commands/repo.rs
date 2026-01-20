use tauri::command;

use crate::errors::{CommandError, Result};

/// Validate that a directory is a git repository.
/// This is called from the frontend before saving the local path.
#[command]
#[specta::specta]
pub async fn validate_git_repo(local_dir: String) -> Result<()> {
    if git2::Repository::open(&local_dir).is_err() {
        return Err(CommandError::bad_input(format!(
            "Directory {} is not a git repository",
            local_dir
        )));
    }
    Ok(())
}
