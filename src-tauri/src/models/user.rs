use serde::Serialize;
use specta::Type;

#[derive(Serialize, Debug, Clone, Type)]
pub struct User {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub name: Option<String>,
}

impl From<octocrab::models::Author> for User {
    fn from(value: octocrab::models::Author) -> Self {
        Self {
            login: value.login,
            id: value.id.0 as u32,
            avatar_url: value.avatar_url.into(),
            gravatar_id: value.gravatar_id,
            name: value.name,
        }
    }
}
