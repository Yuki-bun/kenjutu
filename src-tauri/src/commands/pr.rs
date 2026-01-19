use std::sync::Arc;
use tauri::{command, State};

use crate::db::ReviewedFile;
use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, CommitFileList, GhRepoId, PatchId, SingleFileDiff};
use crate::services::DiffService;
use crate::App;

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    base_sha: String,
    head_sha: String,
) -> Result<Vec<crate::models::PRCommit>> {
    use crate::services::GitService;
    use git2::Oid;

    let mut db = app.get_connection()?;

    let local_repo = db
        .find_local_repo(&repo_id)
        .map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })?
        .ok_or(CommandError::bad_input("Repository not linked"))?;

    let repo_path = local_repo
        .local_dir
        .ok_or(CommandError::bad_input("No local directory set"))?;

    let repository = git2::Repository::open(&repo_path)
        .map_err(|_| CommandError::bad_input("Failed to open repository"))?;

    let head_oid = Oid::from_str(&head_sha).map_err(|err| {
        log::error!("Invalid head SHA: {err}");
        CommandError::bad_input("Invalid head SHA")
    })?;

    let base_oid = Oid::from_str(&base_sha).map_err(|err| {
        log::error!("Invalid base SHA: {err}");
        CommandError::bad_input("Invalid base SHA")
    })?;

    let mut walker = repository.revwalk().map_err(|err| {
        log::error!("Failed to initiate rev walker: {err}");
        CommandError::Internal
    })?;

    let range = format!("{}..{}", base_oid, head_oid);
    walker.push_range(&range).map_err(|err| {
        log::error!("Failed to push range to walker: {err}");
        CommandError::Internal
    })?;

    let mut commits = Vec::new();
    for oid in walker {
        let oid = oid.map_err(|err| {
            log::error!("Walker error: {err}");
            CommandError::Internal
        })?;
        let commit = repository.find_commit(oid).map_err(|err| {
            log::error!("Could not find commit: {err}");
            CommandError::Internal
        })?;

        let change_id = GitService::get_change_id(&commit);

        let pr_commit = crate::models::PRCommit {
            change_id,
            sha: oid.to_string(),
            summary: commit.summary().unwrap_or("").to_string(),
            description: commit.body().unwrap_or("").to_string(),
        };
        commits.push(pr_commit);
    }

    Ok(commits)
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

#[command]
#[specta::specta]
pub async fn get_commit_file_list(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    pr_number: u64,
    commit_sha: String,
) -> Result<CommitFileList> {
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

    let (change_id, files) =
        DiffService::generate_file_list(&repository, &commit_sha, &mut db, &repo_id, pr_number)?;

    Ok(CommitFileList {
        commit_sha,
        change_id,
        files,
    })
}

#[command]
#[specta::specta]
pub async fn get_file_diff(
    app: State<'_, Arc<App>>,
    repo_id: GhRepoId,
    pr_number: u64,
    commit_sha: String,
    file_path: String,
) -> Result<SingleFileDiff> {
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

    DiffService::generate_single_file_diff(
        &repository,
        &commit_sha,
        &file_path,
        &mut db,
        &repo_id,
        pr_number,
    )
}
