use std::str::FromStr;

use git2::Oid;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct CommitId(Oid);

impl std::fmt::Debug for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CommitId {
    pub fn oid(self) -> Oid {
        self.0
    }
}

impl From<Oid> for CommitId {
    fn from(oid: Oid) -> Self {
        Self(oid)
    }
}

impl From<CommitId> for Oid {
    fn from(commit_id: CommitId) -> Self {
        commit_id.0
    }
}

impl FromStr for CommitId {
    type Err = git2::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Oid::from_str(s).map(Self)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CommitId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CommitId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Oid::from_str(&s)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "specta")]
impl specta::Type for CommitId {
    fn inline(
        _type_map: &mut specta::TypeCollection,
        _generics: specta::Generics,
    ) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::PrimitiveType::String)
    }
}
