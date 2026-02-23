use tauri::command;

use super::{Error, Result};
use crate::models::{CommitGraph, JjStatus};
use kenjutu_core::services::{graph, jj};

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
