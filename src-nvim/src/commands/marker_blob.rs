use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;

#[derive(Args)]
pub struct MarkerBlobArgs {
    /// Jujutsu change ID
    #[arg(long)]
    change_id: ChangeId,

    /// Commit SHA
    #[arg(long)]
    commit: CommitId,

    /// File path in the target commit
    #[arg(long)]
    file: PathBuf,
}

pub fn run(local_dir: &Path, args: MarkerBlobArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    match marker.marker_tree().get_path(&args.file) {
        Ok(entry) => {
            let blob = repo
                .find_blob(entry.id())
                .context("failed to read blob from marker tree")?;
            std::io::stdout()
                .write_all(blob.content())
                .context("failed to write blob content")?;
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => {
            // File doesn't exist in marker tree (e.g., newly added file).
            // Output nothing.
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to look up file in marker tree: {e}"
            ));
        }
    }

    Ok(())
}
