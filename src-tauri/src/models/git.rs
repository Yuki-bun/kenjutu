use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Type)]
#[serde(transparent)]
pub struct PatchId(String);

impl PatchId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for PatchId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<PatchId> for String {
    fn from(value: PatchId) -> Self {
        value.0
    }
}
