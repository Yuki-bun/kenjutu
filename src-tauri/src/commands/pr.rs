use tauri::{command, State};

use crate::errors::Result;
use crate::models::{GetPullResponse, PullRequest};
use crate::services::PullRequestService;
use crate::state::AppState;

#[command]
#[specta::specta]
pub async fn get_pull_requests(
    app: State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<Vec<PullRequest>> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    PullRequestService::list_pull_requests(&github, &owner, &repo).await
}

#[command]
#[specta::specta]
pub async fn get_pull(
    app: State<'_, AppState>,
    owner: String,
    repo: String,
    pr: u64,
) -> Result<GetPullResponse> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;
    PullRequestService::get_pull_request_details(&github, &mut db, &owner, &repo, pr).await
}
