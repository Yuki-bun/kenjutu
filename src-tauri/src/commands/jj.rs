use tauri::command;

use super::{Error, Result};
use crate::models::{CommitGraph, JjStatus};
use kenjutu_core::services::{graph, jj};
use kenjutu_types::ChangeId;

/// Get jj status for a directory (is_installed, is_jj_repo)
#[command]
#[specta::specta]
pub async fn get_jj_status(local_dir: String) -> Result<JjStatus> {
    Ok(jj::get_status(&local_dir))
}

/// Get mutable commits from jj log with graph layout
#[command]
#[specta::specta]
pub async fn get_jj_log(local_dir: String) -> Result<CommitGraph> {
    if !jj::is_installed() {
        return Err(Error::bad_input("Jujutsu (jj) is not installed"));
    }
    if !jj::is_jj_repo(&local_dir) {
        return Err(Error::bad_input("Directory is not a jj repository"));
    }
    Ok(graph::get_log_graph(&local_dir)?)
}

/// Describe (set the commit message of) a jj revision.
#[command]
#[specta::specta]
pub async fn describe_commit(
    local_dir: String,
    change_id: ChangeId,
    message: String,
) -> Result<()> {
    if !jj::is_installed() {
        return Err(Error::bad_input("Jujutsu (jj) is not installed"));
    }
    if !jj::is_jj_repo(&local_dir) {
        return Err(Error::bad_input("Directory is not a jj repository"));
    }
    Ok(jj::describe(&local_dir, change_id, &message)?)
}
