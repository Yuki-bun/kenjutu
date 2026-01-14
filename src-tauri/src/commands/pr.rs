use tauri::{command, State};

use crate::db::ReviewedFile;
use crate::errors::{CommandError, Result};
use crate::models::{CommitDiff, GetPullResponse, PullRequest};
use crate::services::{DiffService, PullRequestService};
use crate::state::AppState;

#[command]
#[specta::specta]
pub async fn get_pull_requests(
    app: State<'_, AppState>,
    node_id: String,
) -> Result<Vec<PullRequest>> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;
    PullRequestService::list_pull_requests(&github, &mut db, &node_id).await
}

#[command]
#[specta::specta]
pub async fn get_pull(
    app: State<'_, AppState>,
    node_id: String,
    pr: u64,
) -> Result<GetPullResponse> {
    let app_instance = app.get().await?;
    let github = app_instance.github_service();
    let mut db = app_instance.get_connection().await?;
    PullRequestService::get_pull_request_details(&github, &mut db, &node_id, pr).await
}

#[command]
#[specta::specta]
pub async fn get_commit_diff(
    app: State<'_, AppState>,
    node_id: String,
    pr_number: u64,
    commit_sha: String,
) -> Result<CommitDiff> {
    let app_instance = app.get().await?;
    let mut db = app_instance.get_connection().await?;

    let repo_dir = db
        .find_local_repo(&node_id)
        .await
        .map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })?
        .ok_or_else(|| CommandError::bad_input("Please set local repository to view diff"))?;

    let local_dir = repo_dir
        .local_dir
        .ok_or_else(|| CommandError::bad_input("Please set local repository to view diff"))?;

    let repository = git2::Repository::open(&local_dir).map_err(|err| {
        log::error!("Could not find local repository: {err}");
        CommandError::bad_input(
            "Could not connect to repository set by user. Please reset local repository for this repository",
        )
    })?;

    // Generate diff synchronously (all git2 operations)
    let (change_id, files) = DiffService::generate_diff_sync(&repository, &commit_sha)?;

    // Populate reviewed status asynchronously
    DiffService::populate_reviewed_status(
        commit_sha, change_id, files, &mut db, &node_id, pr_number,
    )
    .await
}

#[command]
#[specta::specta]
pub async fn toggle_file_reviewed(
    app: State<'_, AppState>,
    node_id: String,
    pr_number: u64,
    change_id: Option<String>,
    file_path: String,
    patch_id: String,
    is_reviewed: bool,
) -> Result<()> {
    let app_instance = app.get().await?;
    let mut db = app_instance.get_connection().await?;

    if is_reviewed {
        // CREATE: Insert reviewed file
        let reviewed_file = ReviewedFile {
            github_node_id: node_id.clone(),
            pr_number: pr_number as i64,
            change_id,
            file_path,
            patch_id,
            reviewed_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_reviewed_file(reviewed_file)
            .await
            .map_err(|err| {
                log::error!("Failed to insert reviewed file: {err}");
                CommandError::Internal
            })?;
    } else {
        // DELETE: Remove reviewed file
        db.delete_reviewed_file(
            &node_id,
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
