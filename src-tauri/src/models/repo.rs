use serde::Serialize;
use specta::Type;
use std::path::PathBuf;

use crate::models::GhRepoId;

#[derive(Serialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: GhRepoId,
    pub name: String,
    pub html_url: String,
    pub owner_name: String,
}

impl From<octocrab::models::Repository> for Repo {
    fn from(value: octocrab::models::Repository) -> Self {
        Self {
            id: value
                .node_id
                .expect("Repository node_id should be populated")
                .into(),
            name: value.name,
            html_url: value.html_url.map(|u| u.to_string()).unwrap_or_default(),
            owner_name: value.owner.map(|o| o.login).unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct FullRepo {
    pub name: String,
    pub owner_name: Option<String>,
    pub description: Option<String>,
    pub local_repo: Option<PathBuf>,
}

impl FullRepo {
    pub fn new(octo_repo: octocrab::models::Repository, local_repo: Option<PathBuf>) -> Self {
        Self {
            name: octo_repo.name,
            owner_name: octo_repo.owner.map(|owner| owner.login),
            description: octo_repo.description,
            local_repo,
        }
    }
}
