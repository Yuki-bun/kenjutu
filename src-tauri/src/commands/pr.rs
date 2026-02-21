use std::path::PathBuf;

use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;
use tauri::command;

use super::{Error, Result};
use crate::models::{CommitFileList, DiffLine, FileDiff, HunkId};
use kenjutu_core::services::diff::PartialReviewDiffs;
use kenjutu_core::services::git::get_or_fetch_commit;
use kenjutu_core::services::{diff, git};

#[command]
#[specta::specta]
pub async fn get_commits_in_range(
    local_dir: String,
    base_sha: CommitId,
    head_sha: CommitId,
) -> Result<Vec<crate::models::PRCommit>> {
    let repo = git::open_repository(&local_dir)?;

    get_or_fetch_commit(&repo, base_sha)?;
    get_or_fetch_commit(&repo, head_sha)?;

    let commits = git::get_commits_in_range(&repo, base_sha, head_sha)?;
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
pub async fn get_partial_review_diffs(
    local_dir: String,
    change_id: ChangeId,
    commit_sha: CommitId,
    file_path: String,
    old_path: Option<String>,
) -> Result<PartialReviewDiffs> {
    let repository = git::open_repository(&local_dir)?;
    let file_path = PathBuf::from(file_path);
    let old_path = old_path.map(PathBuf::from);

    Ok(diff::generate_partial_review_diffs(
        &repository,
        commit_sha,
        change_id,
        &file_path,
        old_path.as_deref(),
    )?)
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

#[command]
#[specta::specta]
pub async fn mark_hunk_reviewed(
    local_dir: String,
    change_id: ChangeId,
    sha: CommitId,
    file_path: String,
    old_path: Option<String>,
    hunk: HunkId,
) -> Result<()> {
    let repo = git::open_repository(&local_dir)?;
    let mut marker_commit =
        MarkerCommit::get(&repo, change_id, sha).map_err(|err| Error::MarkerCommit {
            message: format!("Failed to open marker commit: {}", err),
        })?;

    let file_path = PathBuf::from(file_path);
    let old_path = old_path.map(PathBuf::from);
    let hunk: marker_commit::HunkId = hunk.into();

    marker_commit
        .mark_hunk_reviewed(&file_path, old_path.as_deref(), &hunk)
        .map_err(|err| Error::MarkerCommit {
            message: format!("Failed to mark hunk as reviewed: {}", err),
        })?;
    marker_commit.write().map_err(|err| Error::MarkerCommit {
        message: format!("Failed to write marker commit: {}", err),
    })?;

    Ok(())
}

#[command]
#[specta::specta]
pub async fn unmark_hunk_reviewed(
    local_dir: String,
    change_id: ChangeId,
    sha: CommitId,
    file_path: String,
    old_path: Option<String>,
    hunk: HunkId,
) -> Result<()> {
    let repo = git::open_repository(&local_dir)?;
    let mut marker_commit =
        MarkerCommit::get(&repo, change_id, sha).map_err(|err| Error::MarkerCommit {
            message: format!("Failed to open marker commit: {}", err),
        })?;

    let file_path = PathBuf::from(file_path);
    let old_path = old_path.map(PathBuf::from);
    let hunk: marker_commit::HunkId = hunk.into();

    marker_commit
        .unmark_hunk_reviewed(&file_path, old_path.as_deref(), &hunk)
        .map_err(|err| Error::MarkerCommit {
            message: format!("Failed to unmark hunk as reviewed: {}", err),
        })?;
    marker_commit.write().map_err(|err| Error::MarkerCommit {
        message: format!("Failed to write marker commit: {}", err),
    })?;

    Ok(())
}
