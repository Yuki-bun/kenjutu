use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::models::SshSettings;
use kenjutu_core::services::git::{SshCredential, SshCredentialProvider};

const SETTINGS_STORE: &str = "settings.json";
const SSH_SETTINGS_KEY: &str = "ssh";
const DEFAULT_KEY_NAMES: &[&str] = &["id_ed25519", "id_ecdsa", "id_rsa"];

pub struct SshSettingsState(pub Mutex<SshSettings>);

pub fn load_ssh_settings(app: &AppHandle) -> SshSettings {
    let store = app.store(SETTINGS_STORE);
    match store {
        Ok(store) => store
            .get(SSH_SETTINGS_KEY)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to open settings store: {e}");
            SshSettings::default()
        }
    }
}

pub fn save_ssh_settings(app: &AppHandle, settings: &SshSettings) -> Result<(), SshSettingsError> {
    let store = app.store(SETTINGS_STORE).map_err(|_| SshSettingsError)?;
    let value = serde_json::to_value(settings).map_err(|_| SshSettingsError)?;
    store.set(SSH_SETTINGS_KEY, value);
    store.save().map_err(|_| SshSettingsError)?;

    let state = app.state::<SshSettingsState>();
    let mut current = state.0.lock().map_err(|_| SshSettingsError)?;
    *current = settings.clone();

    Ok(())
}

#[derive(Debug)]
pub struct SshSettingsError;

pub fn discover_default_keys() -> Vec<PathBuf> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let ssh_dir = PathBuf::from(home).join(".ssh");
    DEFAULT_KEY_NAMES
        .iter()
        .map(|name| ssh_dir.join(name))
        .filter(|p| p.exists())
        .collect()
}

pub struct AppSshCredentials {
    override_path: Option<PathBuf>,
}

impl AppSshCredentials {
    pub fn from_state(app: &AppHandle) -> Self {
        let override_path = app
            .state::<SshSettingsState>()
            .0
            .lock()
            .ok()
            .and_then(|s| s.private_key_path.as_ref().map(PathBuf::from));
        Self { override_path }
    }
}

impl SshCredentialProvider for AppSshCredentials {
    fn ssh_credentials(&self) -> Vec<SshCredential> {
        let mut creds = Vec::new();
        if let Some(path) = &self.override_path {
            creds.push(SshCredential::KeyFile(path.clone()));
        }
        creds.push(SshCredential::Agent);
        creds.extend(
            discover_default_keys()
                .into_iter()
                .map(SshCredential::KeyFile),
        );
        creds
    }
}
