use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;

#[derive(Args)]
pub struct MarkFileArgs {
    /// Jujutsu change ID
    #[arg(long)]
    change_id: ChangeId,

    /// Commit SHA
    #[arg(long)]
    commit: CommitId,

    /// File path in the target commit
    #[arg(long)]
    file: PathBuf,

    /// Old file path (for renames)
    #[arg(long)]
    old_path: Option<PathBuf>,
}

#[derive(Args)]
pub struct UnmarkFileArgs {
    /// Jujutsu change ID
    #[arg(long)]
    change_id: ChangeId,

    /// Commit SHA
    #[arg(long)]
    commit: CommitId,

    /// File path in the target commit
    #[arg(long)]
    file: PathBuf,

    /// Old file path (for renames)
    #[arg(long)]
    old_path: Option<PathBuf>,
}

pub fn run_mark(local_dir: &Path, args: MarkFileArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    marker
        .mark_file_reviewed(&args.file, args.old_path.as_deref())
        .map_err(|e| anyhow::anyhow!("failed to mark file reviewed: {e}"))?;

    marker
        .write()
        .map_err(|e| anyhow::anyhow!("failed to write marker commit: {e}"))?;

    println!("{}", serde_json::json!({ "success": true }));
    Ok(())
}

pub fn run_unmark(local_dir: &Path, args: UnmarkFileArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    marker
        .unmark_file_reviewed(&args.file, args.old_path.as_deref())
        .map_err(|e| anyhow::anyhow!("failed to unmark file reviewed: {e}"))?;

    marker
        .write()
        .map_err(|e| anyhow::anyhow!("failed to write marker commit: {e}"))?;

    println!("{}", serde_json::json!({ "success": true }));
    Ok(())
}
