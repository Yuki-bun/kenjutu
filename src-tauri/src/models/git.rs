use std::fmt::Display;

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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Type)]
#[serde(transparent)]
pub struct ChangeId(String);

impl ChangeId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for ChangeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<ChangeId> for String {
    fn from(value: ChangeId) -> Self {
        value.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Type)]
#[serde(transparent)]
pub struct GhRepoId(String);

impl Display for GhRepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = &self.0;
        write!(f, "{inner}")
    }
}

impl GhRepoId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GhRepoId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<GhRepoId> for String {
    fn from(value: GhRepoId) -> Self {
        value.0
    }
}
