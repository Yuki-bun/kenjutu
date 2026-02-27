use serde::{Deserialize, Serialize};
use specta::Type;

/// SSH settings stored in Tauri plugin-store and managed as app state.
#[derive(Debug, Serialize, Deserialize, Type, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SshSettings {
    /// If set, this private key path is tried first before auto-detection.
    pub private_key_path: Option<String>,
}
