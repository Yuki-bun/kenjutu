use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;

#[derive(Args)]
pub struct SetBlobArgs {
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

pub fn run(local_dir: &Path, args: SetBlobArgs) -> Result<()> {
    let mut content = Vec::new();
    std::io::stdin()
        .read_to_end(&mut content)
        .context("failed to read blob content from stdin")?;

    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    marker
        .set_blob(&args.file, args.old_path.as_deref(), &content)
        .map_err(|e| anyhow::anyhow!("failed to set blob: {e}"))?;

    marker
        .write()
        .map_err(|e| anyhow::anyhow!("failed to write marker commit: {e}"))?;

    println!("{}", serde_json::json!({ "success": true }));
    Ok(())
}
