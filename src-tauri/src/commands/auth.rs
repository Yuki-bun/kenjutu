use std::sync::Arc;
use tauri::{command, AppHandle, State};

use crate::errors::Result;
use crate::services::AuthService;
use crate::App;

#[command]
#[specta::specta]
pub async fn auth_github(app_handle: AppHandle, app: State<'_, Arc<App>>) -> Result<()> {
    let app = app.inner().clone();
    AuthService::init_auth_flow(&app_handle, app)
}
