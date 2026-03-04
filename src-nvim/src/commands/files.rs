use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_core::models::FileEntry;
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use serde::Serialize;

#[derive(Args)]
pub struct FilesArgs {
    /// Commit SHA
    #[arg(long)]
    commit: CommitId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    change_id: ChangeId,
    files: Vec<FileEntry>,
}

pub fn run(local_dir: &Path, args: FilesArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let (change_id, files) =
        diff::generate_file_list(&repo, args.commit).context("failed to generate file list")?;

    let output = Output { change_id, files };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
