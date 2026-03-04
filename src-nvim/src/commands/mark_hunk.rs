use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::{HunkId, MarkerCommit};

#[derive(Args)]
pub struct MarkHunkArgs {
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

    /// Hunk old start line
    #[arg(long)]
    old_start: u32,

    /// Hunk old line count
    #[arg(long)]
    old_lines: u32,

    /// Hunk new start line
    #[arg(long)]
    new_start: u32,

    /// Hunk new line count
    #[arg(long)]
    new_lines: u32,
}

#[derive(Args)]
pub struct UnmarkHunkArgs {
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

    /// Hunk old start line
    #[arg(long)]
    old_start: u32,

    /// Hunk old line count
    #[arg(long)]
    old_lines: u32,

    /// Hunk new start line
    #[arg(long)]
    new_start: u32,

    /// Hunk new line count
    #[arg(long)]
    new_lines: u32,
}

pub fn run_mark(local_dir: &Path, args: MarkHunkArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    let hunk = HunkId {
        old_start: args.old_start,
        old_lines: args.old_lines,
        new_start: args.new_start,
        new_lines: args.new_lines,
    };

    marker
        .mark_hunk_reviewed(&args.file, args.old_path.as_deref(), &hunk)
        .map_err(|e| anyhow::anyhow!("failed to mark hunk reviewed: {e}"))?;

    marker
        .write()
        .map_err(|e| anyhow::anyhow!("failed to write marker commit: {e}"))?;

    println!("{}", serde_json::json!({ "success": true }));
    Ok(())
}

pub fn run_unmark(local_dir: &Path, args: UnmarkHunkArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    let hunk = HunkId {
        old_start: args.old_start,
        old_lines: args.old_lines,
        new_start: args.new_start,
        new_lines: args.new_lines,
    };

    marker
        .unmark_hunk_reviewed(&args.file, args.old_path.as_deref(), &hunk)
        .map_err(|e| anyhow::anyhow!("failed to unmark hunk reviewed: {e}"))?;

    marker
        .write()
        .map_err(|e| anyhow::anyhow!("failed to write marker commit: {e}"))?;

    println!("{}", serde_json::json!({ "success": true }));
    Ok(())
}
