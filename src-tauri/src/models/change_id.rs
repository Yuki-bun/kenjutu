use serde::{Deserialize, Serialize};
use specta::{
    datatype::{DataType, PrimitiveType},
    Generics, TypeCollection,
};

#[derive(Debug)]
pub struct InvalidChangeIdError {
    received: String,
}

impl std::fmt::Display for InvalidChangeIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid ChangeId: expected a 32-character string, got '{}'",
            self.received
        )
    }
}

impl std::error::Error for InvalidChangeIdError {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash, Copy)]
#[serde(into = "String", try_from = "String")]
pub struct ChangeId([u8; 32]);

impl specta::Type for ChangeId {
    fn inline(_type_map: &mut TypeCollection, _generics: Generics) -> DataType {
        DataType::Primitive(PrimitiveType::String)
    }
}

impl std::fmt::Debug for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl TryFrom<&str> for ChangeId {
    type Error = InvalidChangeIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        if bytes.len() != 32 {
            return Err(InvalidChangeIdError {
                received: value.to_string(),
            });
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl TryFrom<String> for ChangeId {
    type Error = InvalidChangeIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<ChangeId> for String {
    fn from(value: ChangeId) -> Self {
        std::str::from_utf8(&value.0).unwrap().to_string()
    }
}

impl From<ChangeId> for marker_commit::ChangeId {
    fn from(val: ChangeId) -> Self {
        marker_commit::ChangeId::from(val.0)
    }
}
