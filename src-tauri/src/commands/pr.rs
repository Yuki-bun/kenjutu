use tauri::command;

use super::Result;
use crate::db::{RepoDb, ReviewedFileRepository};
use crate::models::{ChangeId, CommitFileList, DiffHunk, PatchId};
use crate::services::{diff, git, jj};

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: String,
    head_sha: String,
) -> Result<Vec<crate::models::PRCommit>> {
    Ok(jj::get_commits_in_range(&local_dir, &base_sha, &head_sha)?)
}

#[command]
#[specta::specta]
pub async fn toggle_file_reviewed(
    local_dir: String,
    change_id: ChangeId,
    file_path: String,
    patch_id: PatchId,
    is_reviewed: bool,
) -> Result<()> {
    let repository = git::open_repository(&local_dir)?;
    let db = RepoDb::open(&repository)?;
    let review_repo = ReviewedFileRepository::new(&db);

    if is_reviewed {
        review_repo.mark_file_reviewed(change_id, file_path, patch_id)?;
    } else {
        review_repo.mark_file_not_reviewed(&change_id, &file_path)?;
    }

    Ok(())
}

#[command]
#[specta::specta]
pub async fn get_commit_file_list(local_dir: String, commit_sha: String) -> Result<CommitFileList> {
    let repository = git::open_repository(&local_dir)?;
    let db = RepoDb::open(&repository)?;
    let review_repo = ReviewedFileRepository::new(&db);

    let (change_id, files) = diff::generate_file_list(&repository, &commit_sha, &review_repo)?;

    Ok(CommitFileList {
        commit_sha,
        change_id,
        files,
    })
}

#[command]
#[specta::specta]
pub async fn get_file_diff(
    local_dir: String,
    commit_sha: String,
    file_path: String,
    old_path: Option<String>,
) -> Result<Vec<DiffHunk>> {
    let repository = git::open_repository(&local_dir)?;

    Ok(diff::generate_single_file_diff(
        &repository,
        &commit_sha,
        &file_path,
        old_path.as_deref(),
    )?)
}

#[command]
#[specta::specta]
pub async fn get_change_id_from_sha(local_dir: String, sha: String) -> Result<Option<ChangeId>> {
    let repository = git::open_repository(&local_dir)?;
    let oid = git2::Oid::from_str(&sha)
        .map_err(|_| crate::services::git::Error::InvalidSha(sha.clone()))?;
    let commit = repository
        .find_commit(oid)
        .map_err(|_| crate::services::git::Error::CommitNotFound(sha))?;

    Ok(git::get_change_id(&commit))
}
