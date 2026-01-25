use tauri::{command, AppHandle};

use super::Result;
use crate::services::AuthService;

#[command]
#[specta::specta]
pub async fn auth_github(app_handle: AppHandle) -> Result<()> {
    Ok(AuthService::init_auth_flow(&app_handle)?)
}
