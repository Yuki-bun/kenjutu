use tauri::{command, AppHandle};

use crate::errors::Result;
use crate::services::AuthService;

#[command]
#[specta::specta]
pub async fn auth_github(app_handle: AppHandle) -> Result<()> {
    AuthService::init_auth_flow(&app_handle)
}
