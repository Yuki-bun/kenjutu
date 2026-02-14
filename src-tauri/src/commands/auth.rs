use tauri::{command, AppHandle};

use super::Result;
use crate::{models::DeviceFlowInfo, services::auth};

#[command]
#[specta::specta]
pub async fn auth_github(app_handle: AppHandle) -> Result<DeviceFlowInfo> {
    Ok(auth::init_auth_flow(&app_handle).await?)
}
