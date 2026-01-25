use tauri::command;

use super::Result;
use crate::db::RepoDb;
use crate::models::{ChangeId, CommitFileList, PatchId, SingleFileDiff};
use crate::services::{DiffService, GitService, ReviewRepository};

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: String,
    head_sha: String,
) -> Result<Vec<crate::models::PRCommit>> {
    let repository = GitService::open_repository(&local_dir)?;
    Ok(GitService::get_commits_in_range(
        &repository,
        &base_sha,
        &head_sha,
    )?)
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
    let repository = GitService::open_repository(&local_dir)?;
    let mut db = RepoDb::open(&repository)?;
    let mut review_repo = ReviewRepository::new(&mut db);

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
    let repository = GitService::open_repository(&local_dir)?;
    let mut db = RepoDb::open(&repository)?;
    let mut review_repo = ReviewRepository::new(&mut db);

    let (change_id, files) =
        DiffService::generate_file_list(&repository, &commit_sha, &mut review_repo)?;

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
) -> Result<SingleFileDiff> {
    let repository = GitService::open_repository(&local_dir)?;
    let mut db = RepoDb::open(&repository)?;
    let mut review_repo = ReviewRepository::new(&mut db);

    Ok(DiffService::generate_single_file_diff(
        &repository,
        &commit_sha,
        &file_path,
        &mut review_repo,
    )?)
}
