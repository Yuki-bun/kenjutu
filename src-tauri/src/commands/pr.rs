use std::sync::Arc;
use tauri::{command, State};

use crate::db::ReviewedFile;
use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, CommitDiff, GetPullResponse, GhRepoId, PatchId};
use crate::services::{DiffService, PullRequestService};
use crate::App;

#[command]
#[specta::specta]
pub async fn get_pull(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    pr: u64,
) -> Result<GetPullResponse> {
    let github = app.github_service();
    let mut db = app.get_connection()?;
    PullRequestService::get_pull_request_details(&github, &mut db, &repo_id, pr).await
}

#[command]
#[specta::specta]
pub async fn get_commit_diff(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    pr_number: u64,
    commit_sha: String,
) -> Result<CommitDiff> {
    let mut db = app.get_connection()?;

    let repo_dir = db
        .find_local_repo(&repo_id)
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
    let (change_id, files) =
        DiffService::generate_diff(&repository, &commit_sha, &mut db, &repo_id, pr_number)?;

    Ok(CommitDiff {
        commit_sha,
        change_id,
        files,
    })
}

#[command]
#[specta::specta]
pub async fn toggle_file_reviewed(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    pr_number: u64,
    change_id: Option<ChangeId>,
    file_path: String,
    patch_id: PatchId,
    is_reviewed: bool,
) -> Result<()> {
    let mut db = app.get_connection()?;

    if is_reviewed {
        // CREATE: Insert reviewed file
        let reviewed_file = ReviewedFile {
            gh_repo_id: repo_id,
            pr_number: pr_number as i64,
            change_id,
            file_path,
            patch_id,
            reviewed_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_reviewed_file(reviewed_file).map_err(|err| {
            log::error!("Failed to insert reviewed file: {err}");
            CommandError::Internal
        })?;
    } else {
        // DELETE: Remove reviewed file
        db.delete_reviewed_file(
            &repo_id,
            pr_number as i64,
            change_id.as_ref(),
            &file_path,
            &patch_id,
        )
        .map_err(|err| {
            log::error!("Failed to delete reviewed file: {err}");
            CommandError::Internal
        })?;
    }

    Ok(())
}
