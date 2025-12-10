use serde::Serialize;
use specta::Type;

use super::User;

#[derive(Serialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub github_url: Option<String>,
    pub id: u64,
    pub title: Option<String>,
    pub author: Option<User>,
    pub number: u64,
}

impl From<octocrab::models::pulls::PullRequest> for PullRequest {
    fn from(value: octocrab::models::pulls::PullRequest) -> Self {
        Self {
            github_url: value.html_url.map(|url| url.into()),
            id: value.id.0,
            title: value.title,
            author: value.user.map(|owner| User::from(*owner)),
            number: value.number,
        }
    }
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetPullResponse {
    pub title: String,
    pub body: String,
    pub base_branch: String,
    pub head_branch: String,
    pub commits: Vec<PRCommit>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PRCommit {
    pub change_id: Option<String>,
    pub sha: String,
    pub summary: String,
    pub description: String,
}
