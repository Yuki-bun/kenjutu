use tauri::{command, State};

use crate::errors::{CommandError, Result};
use crate::models::{CommitDiff, GetPullResponse, PullRequest};
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

#[command]
#[specta::specta]
pub async fn get_commit_diff(
    app: State<'_, AppState>,
    owner: String,
    repo: String,
    commit_sha: String,
) -> Result<CommitDiff> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;

    // Get repository node_id and local path
    let gh_repo = github.get_repository(&owner, &repo).await?;
    let repo_node_id = gh_repo.node_id.ok_or_else(|| {
        log::error!("Got null node id");
        CommandError::Internal
    })?;

    let repo_dir = db
        .find_local_repo(&repo_node_id)
        .await
        .map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })?
        .ok_or_else(|| CommandError::bad_input("Please set local repository to view diff"))?;

    let repository = git2::Repository::open(&repo_dir.local_dir).map_err(|err| {
        log::error!("Could not find local repository: {err}");
        CommandError::bad_input(
            "Could not connect to repository set by user. Please reset local repository for this repository",
        )
    })?;

    PullRequestService::get_commit_diff(&repository, &commit_sha)
}
