use tauri::{command, AppHandle, Manager};

use super::{Error, Result};
use crate::models::SshSettings;
use crate::services::ssh::{save_ssh_settings, SshSettingsState};

#[command]
#[specta::specta]
pub async fn get_ssh_settings(app: AppHandle) -> Result<SshSettings> {
    let state = app.state::<SshSettingsState>();
    let settings = state.0.lock().map_err(|_| Error::Internal)?;
    Ok(settings.clone())
}

#[command]
#[specta::specta]
pub async fn set_ssh_settings(app: AppHandle, settings: SshSettings) -> Result<()> {
    save_ssh_settings(&app, &settings).map_err(|_| Error::Internal)?;
    Ok(())
}
