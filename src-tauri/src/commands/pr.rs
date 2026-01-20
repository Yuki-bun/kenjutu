use std::sync::Arc;
use tauri::{command, State};

use crate::db::ReviewedFile;
use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, CommitFileList, PatchId, SingleFileDiff};
use crate::services::DiffService;
use crate::App;

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: String,
    head_sha: String,
) -> Result<Vec<crate::models::PRCommit>> {
    use crate::services::GitService;
    use git2::Oid;

    let repository = git2::Repository::open(&local_dir).map_err(|err| {
        log::error!("Could not open repository: {err}");
        CommandError::bad_input("Failed to open repository")
    })?;

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
    local_dir: String,
    change_id: ChangeId,
    file_path: String,
    patch_id: PatchId,
    is_reviewed: bool,
) -> Result<()> {
    let repository = git2::Repository::open(&local_dir).map_err(|err| {
        log::error!("Could not open repository: {err}");
        CommandError::bad_input("Failed to open repository")
    })?;

    let mut db = app.get_repo_db(&repository)?;

    if is_reviewed {
        let reviewed_file = ReviewedFile {
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
        db.delete_reviewed_file(&change_id, &file_path)
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
    local_dir: String,
    commit_sha: String,
) -> Result<CommitFileList> {
    let repository = git2::Repository::open(&local_dir).map_err(|err| {
        log::error!("Could not open repository: {err}");
        CommandError::bad_input(
            "Could not connect to repository. Please check the local repository path.",
        )
    })?;

    let mut db = app.get_repo_db(&repository)?;

    let (change_id, files) = DiffService::generate_file_list(&repository, &commit_sha, &mut db)?;

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
    local_dir: String,
    commit_sha: String,
    file_path: String,
) -> Result<SingleFileDiff> {
    let repository = git2::Repository::open(&local_dir).map_err(|err| {
        log::error!("Could not open repository: {err}");
        CommandError::bad_input(
            "Could not connect to repository. Please check the local repository path.",
        )
    })?;

    let mut db = app.get_repo_db(&repository)?;

    DiffService::generate_single_file_diff(&repository, &commit_sha, &file_path, &mut db)
}
