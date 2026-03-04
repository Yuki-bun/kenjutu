use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;

#[derive(Clone, ValueEnum)]
pub enum TreeKind {
    Base,
    Marker,
    Target,
}

#[derive(Args)]
pub struct BlobArgs {
    /// Jujutsu change ID
    #[arg(long)]
    change_id: ChangeId,

    /// Commit SHA
    #[arg(long)]
    commit: CommitId,

    /// File path in the target commit
    #[arg(long)]
    file: PathBuf,

    /// Old file path (for renames — where the file lived in the base commit)
    #[arg(long)]
    old_path: Option<PathBuf>,

    /// Which tree to read from
    #[arg(long)]
    tree: TreeKind,
}

pub fn run(local_dir: &Path, args: BlobArgs) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let marker = MarkerCommit::get(&repo, args.change_id, args.commit)
        .map_err(|e| anyhow::anyhow!("failed to get marker commit: {e}"))?;

    let tree = match args.tree {
        TreeKind::Base => marker.base_tree(),
        TreeKind::Marker => marker.marker_tree(),
        TreeKind::Target => marker.target_tree(),
    };

    // Determine the lookup path.
    // - target tree: always use `file`
    // - base tree: use `old_path` if provided (file was at old location before rename)
    // - marker tree: try `file` first, fall back to `old_path` (marker may not
    //   have been updated for renames yet)
    let lookup_path = match args.tree {
        TreeKind::Target => &args.file,
        TreeKind::Base => args.old_path.as_ref().unwrap_or(&args.file),
        TreeKind::Marker => &args.file,
    };

    match tree.get_path(lookup_path) {
        Ok(entry) => {
            let blob = repo
                .find_blob(entry.id())
                .context("failed to read blob from tree")?;
            std::io::stdout()
                .write_all(blob.content())
                .context("failed to write blob content")?;
        }
        Err(_) if matches!(args.tree, TreeKind::Marker) => {
            // For marker tree, try old_path as fallback (rename not yet applied)
            if let Some(ref old_path) = args.old_path {
                match tree.get_path(old_path) {
                    Ok(entry) => {
                        let blob = repo
                            .find_blob(entry.id())
                            .context("failed to read blob from tree")?;
                        std::io::stdout()
                            .write_all(blob.content())
                            .context("failed to write blob content")?;
                    }
                    Err(_) => {
                        // File doesn't exist in marker tree at all — output nothing
                    }
                }
            }
            // File doesn't exist in marker tree — output nothing
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => {
            // File doesn't exist in this tree — output nothing
        }
        Err(e) => {
            return Err(anyhow::anyhow!("failed to look up file in tree: {e}"));
        }
    }

    Ok(())
}
