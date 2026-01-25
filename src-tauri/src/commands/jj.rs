use tauri::command;

use super::{Error, Result};
use crate::models::{JjLogResult, JjStatus};
use crate::services::JjService;

/// Get jj status for a directory (is_installed, is_jj_repo)
#[command]
#[specta::specta]
pub async fn get_jj_status(local_dir: String) -> Result<JjStatus> {
    Ok(JjService::get_status(&local_dir))
}

/// Get mutable commits from jj log
#[command]
#[specta::specta]
pub async fn get_jj_log(local_dir: String) -> Result<JjLogResult> {
    if !JjService::is_installed() {
        return Err(Error::bad_input("Jujutsu (jj) is not installed"));
    }
    if !JjService::is_jj_repo(&local_dir) {
        return Err(Error::bad_input("Directory is not a jj repository"));
    }
    Ok(JjService::get_log(&local_dir)?)
}
