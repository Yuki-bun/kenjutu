use std::path::PathBuf;

use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;
use tauri::command;

use super::{Error, Result};
use crate::models::{CommitFileList, DiffLine, FileDiff};
use kenjutu_core::services::git::{get_or_fetch_commit, store_commit_as_fake_remote};
use kenjutu_core::services::{diff, git, jj};

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: CommitId,
    head_sha: CommitId,
) -> Result<Vec<crate::models::PRCommit>> {
    let repo = git::open_repository(&local_dir)?;

    // Ensure both commits are in the repo
    let head_commit = get_or_fetch_commit(&repo, base_sha)?;
    let base_commit = get_or_fetch_commit(&repo, head_sha)?;

    // Ensure jj can find the commits by storing them under refs/remotes/kenjutu
    // and the refs are dropped at the end of this function
    let _head_ref = store_commit_as_fake_remote(&repo, &head_commit)?;
    let _base_ref = store_commit_as_fake_remote(&repo, &base_commit)?;

    let commits = jj::get_commits_in_range(&local_dir, base_sha, head_sha);

    let commits = commits?;
    Ok(commits)
}

#[command]
#[specta::specta]
pub async fn toggle_file_reviewed(
    local_dir: String,
    change_id: ChangeId,
    sha: CommitId,
    file_path: String,
    old_path: Option<String>,
    is_reviewed: bool,
) -> Result<()> {
    let repo = git::open_repository(&local_dir)?;
    let mut marker_commit =
        MarkerCommit::get(&repo, change_id, sha).map_err(|err| Error::MarkerCommit {
            message: format!("Failed to open marker commit: {}", err),
        })?;

    let file_path = PathBuf::from(file_path);
    let old_path = old_path.map(PathBuf::from);
    let old_path = old_path.as_ref().map(|path| path.as_ref());

    if is_reviewed {
        marker_commit
            .mark_file_reviewed(&file_path, old_path)
            .map_err(|err| Error::MarkerCommit {
                message: format!("Failed to mark commit as marked: {}", err),
            })?;
    } else {
        marker_commit
            .unmark_file_reviewed(&file_path, old_path)
            .map_err(|err| Error::MarkerCommit {
                message: format!("Failed to mark commit as marked: {}", err),
            })?;
    }
    marker_commit.write().map_err(|err| Error::MarkerCommit {
        message: format!("Failed to write marker commit: {}", err),
    })?;

    Ok(())
}

#[command]
#[specta::specta]
pub async fn get_commit_file_list(
    local_dir: String,
    commit_sha: CommitId,
) -> Result<CommitFileList> {
    let repository = git::open_repository(&local_dir)?;

    let (change_id, files) = diff::generate_file_list(&repository, commit_sha)?;

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
    commit_sha: CommitId,
    file_path: String,
    old_path: Option<String>,
) -> Result<FileDiff> {
    let repository = git::open_repository(&local_dir)?;
    let file_path = PathBuf::from(file_path);
    let old_path = old_path.map(PathBuf::from);

    Ok(diff::generate_single_file_diff(
        &repository,
        commit_sha,
        &file_path,
        old_path.as_deref(),
    )?)
}

#[command]
#[specta::specta]
pub async fn get_change_id_from_sha(local_dir: String, sha: CommitId) -> Result<Option<ChangeId>> {
    let repository = git::open_repository(&local_dir)?;
    let commit = get_or_fetch_commit(&repository, sha)?;
    Ok(git::get_change_id(&commit))
}

#[command]
#[specta::specta]
pub async fn get_context_lines(
    local_dir: String,
    commit_sha: CommitId,
    file_path: String,
    start_line: u32,
    end_line: u32,
    old_start_line: u32,
) -> Result<Vec<DiffLine>> {
    let repository = git::open_repository(&local_dir)?;

    Ok(diff::get_context_lines(
        &repository,
        commit_sha,
        &file_path,
        start_line,
        end_line,
        old_start_line,
    )?)
}
