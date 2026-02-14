use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Type, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowInfo {
    pub user_code: String,
    pub verification_uri: String,
}
