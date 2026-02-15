use tauri::command;

use super::{Error, Result};
use crate::db::{RepoDb, ReviewedFileRepository};
use crate::models::{ChangeId, CommitFileList, DiffLine, FileDiff, PatchId};
use crate::services::git::{get_or_fetch_commit, store_commit_as_fake_remote};
use crate::services::{diff, git, jj};

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: String,
    head_sha: String,
) -> Result<Vec<crate::models::PRCommit>> {
    let repo = git::open_repository(&local_dir)?;
    let head_oid = oid_from_str(&head_sha)?;
    let base_oid = oid_from_str(&base_sha)?;

    // Ensure both commits are in the repo
    let head_commit = get_or_fetch_commit(&repo, head_oid)?;
    let base_commit = get_or_fetch_commit(&repo, base_oid)?;

    // Ensure jj can find the commits by storing them under refs/remotes/revue
    // and the refs are dropped at the end of this function
    let _head_ref = store_commit_as_fake_remote(&repo, &head_commit)?;
    let _base_ref = store_commit_as_fake_remote(&repo, &base_commit)?;

    let commits = jj::get_commits_in_range(&local_dir, &base_sha, &head_sha);

    let commits = commits?;
    Ok(commits)
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
    let oid = oid_from_str(&commit_sha)?;

    let (change_id, files) = diff::generate_file_list(&repository, oid, &review_repo)?;

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
) -> Result<FileDiff> {
    let repository = git::open_repository(&local_dir)?;
    let oid = oid_from_str(&commit_sha)?;

    Ok(diff::generate_single_file_diff(
        &repository,
        oid,
        &file_path,
        old_path.as_deref(),
    )?)
}

#[command]
#[specta::specta]
pub async fn get_change_id_from_sha(local_dir: String, sha: String) -> Result<Option<ChangeId>> {
    let repository = git::open_repository(&local_dir)?;
    let oid = oid_from_str(&sha)?;
    let commit = get_or_fetch_commit(&repository, oid)?;
    Ok(git::get_change_id(&commit))
}

#[command]
#[specta::specta]
pub async fn get_context_lines(
    local_dir: String,
    commit_sha: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
    old_start_line: u32,
) -> Result<Vec<DiffLine>> {
    let repository = git::open_repository(&local_dir)?;
    let oid = oid_from_str(&commit_sha)?;

    Ok(diff::get_context_lines(
        &repository,
        oid,
        &file_path,
        start_line,
        end_line,
        old_start_line,
    )?)
}

fn oid_from_str(s: &str) -> Result<git2::Oid> {
    git2::Oid::from_str(s).map_err(|_| Error::bad_input(format!("Invalid SHA: {s}")))
}
