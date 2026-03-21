use std::str::FromStr;

use git2::Commit;

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

#[derive(Clone, Eq, PartialEq, Hash, Copy)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "String")
)]
pub struct ChangeId([u8; 32]);

#[cfg(feature = "specta")]
impl specta::Type for ChangeId {
    fn inline(
        _type_map: &mut specta::TypeCollection,
        _generics: specta::Generics,
    ) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::PrimitiveType::String)
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

impl FromStr for ChangeId {
    type Err = InvalidChangeIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != 32 {
            return Err(InvalidChangeIdError {
                received: s.to_string(),
            });
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl TryFrom<&str> for ChangeId {
    type Error = InvalidChangeIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for ChangeId {
    type Error = InvalidChangeIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<ChangeId> for String {
    fn from(value: ChangeId) -> Self {
        std::str::from_utf8(&value.0).unwrap().to_string()
    }
}

impl From<[u8; 32]> for ChangeId {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

pub trait CommitChangeIdExt {
    fn change_id(&self) -> ChangeId;
}

impl CommitChangeIdExt for Commit<'_> {
    fn change_id(&self) -> ChangeId {
        extract_change_id(self).unwrap_or_else(|| synthetic_change_id(self.id()))
    }
}

fn extract_change_id(commit: &Commit) -> Option<ChangeId> {
    let buf = commit.header_field_bytes("change-id").ok()?;
    let Ok(bytes): Result<[u8; 32], _> = buf.as_ref().try_into() else {
        log::warn!(
            "found invalid change-id header in commit: {}, value: {:?}",
            commit.id(),
            buf.as_str()
        );
        return None;
    };
    Some(ChangeId::from(bytes))
}

const REVERSE_HEX_CHARS: &[u8; 16] = b"zyxwvutsrqponmlk";

fn synthetic_change_id(sha: git2::Oid) -> ChangeId {
    let sha_bytes = sha.as_bytes();
    let mut encoded = [0u8; 32];
    for (i, b) in sha_bytes[4..20]
        .iter()
        .rev()
        .map(|b| b.reverse_bits())
        .enumerate()
    {
        encoded[i * 2] = REVERSE_HEX_CHARS[(b >> 4) as usize];
        encoded[i * 2 + 1] = REVERSE_HEX_CHARS[(b & 0x0f) as usize];
    }
    ChangeId::from(encoded)
}
