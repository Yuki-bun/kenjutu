use std::{path::Path, process::Command};

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_core::models::FileEntry;
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use serde::Serialize;

#[derive(Args)]
pub struct FilesArgs {
    /// JJ change ID
    #[arg(long)]
    change_id: ChangeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    change_id: ChangeId,
    commit_id: CommitId,
    files: Vec<FileEntry>,
}

pub fn run(local_dir: &Path, args: FilesArgs) -> Result<()> {
    let commit_id = find_commit_from_change_id(local_dir, &args.change_id)
        .context("failed to find commit ID from change ID")?;

    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let (change_id, files) =
        diff::generate_file_list(&repo, commit_id).context("failed to generate file list")?;

    let output = Output {
        commit_id,
        change_id,
        files,
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

fn find_commit_from_change_id(dir: &Path, change_id: &ChangeId) -> Result<CommitId> {
    let output = Command::new("jj")
        .args([
            "log",
            "-r",
            &change_id.to_string(),
            "-T",
            "commit_id",
            "--no-graph",
        ])
        .current_dir(dir)
        .output()
        .context("failed to execute jj log command")?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let commit_id_str = stdout.trim();
        Ok(commit_id_str.parse()?)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "jj log failed with status {}: {}",
            output.status,
            stderr.trim()
        ))
    }
}
