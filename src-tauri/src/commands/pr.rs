use tauri::{command, State};

use crate::db::ReviewedFile;
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
    pr_number: u64,
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

    // Generate diff synchronously (all git2 operations)
    let (change_id, files) = PullRequestService::generate_diff_sync(&repository, &commit_sha)?;

    // Populate reviewed status asynchronously
    PullRequestService::populate_reviewed_status(
        commit_sha,
        change_id,
        files,
        &mut db,
        &repo_node_id,
        pr_number,
    )
    .await
}

#[command]
#[specta::specta]
pub async fn toggle_file_reviewed(
    app: State<'_, AppState>,
    owner: String,
    repo: String,
    pr_number: u64,
    change_id: Option<String>,
    file_path: String,
    patch_id: String,
    is_reviewed: bool,
) -> Result<()> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;

    // Get github_node_id
    let gh_repo = github.get_repository(&owner, &repo).await?;
    let github_node_id = gh_repo.node_id.ok_or(CommandError::Internal)?;

    if is_reviewed {
        // CREATE: Insert reviewed file
        let reviewed_file = ReviewedFile {
            github_node_id,
            pr_number: pr_number as i64,
            change_id,
            file_path,
            patch_id,
            reviewed_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_reviewed_file(reviewed_file).await.map_err(|err| {
            log::error!("Failed to insert reviewed file: {err}");
            CommandError::Internal
        })?;
    } else {
        // DELETE: Remove reviewed file
        db.delete_reviewed_file(
            &github_node_id,
            pr_number as i64,
            change_id.as_deref(),
            &file_path,
            &patch_id,
        )
        .await
        .map_err(|err| {
            log::error!("Failed to delete reviewed file: {err}");
            CommandError::Internal
        })?;
    }

    Ok(())
}
