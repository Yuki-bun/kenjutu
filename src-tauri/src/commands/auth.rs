use tauri::{command, AppHandle};

use super::Result;
use crate::services::auth;

#[command]
#[specta::specta]
pub async fn auth_github(app_handle: AppHandle) -> Result<()> {
    Ok(auth::init_auth_flow(&app_handle)?)
}
