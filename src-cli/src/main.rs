mod output;
mod resolve;

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use comment_commit::get_all_ported_comments;
use kenjutu_types::ChangeId;

use crate::output::{CommentOutput, FileComments, Output};

#[derive(Parser)]
#[command(
    name = "kenjutu-comments",
    about = "Retrieve inline diff comments for a jj change"
)]
struct Cli {
    /// Path to the repository directory
    #[arg(short, long, default_value = ".")]
    dir: String,

    /// Jujutsu change ID (auto-detected from working copy if omitted)
    #[arg(short, long)]
    change_id: Option<String>,

    /// Filter to a specific file path
    #[arg(short, long)]
    file: Option<String>,

    /// Include resolved comments (default: unresolved only)
    #[arg(short, long, default_value_t = false)]
    all: bool,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        let err = serde_json::json!({ "error": format!("{e:#}") });
        eprintln!("{}", serde_json::to_string(&err).unwrap());
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let local_dir = std::fs::canonicalize(&cli.dir).context("invalid directory")?;

    let change_id: ChangeId = match cli.change_id {
        Some(raw) => ChangeId::try_from(raw.as_str())
            .map_err(|e| anyhow::anyhow!("invalid --change-id: {e}"))?,
        None => resolve::auto_detect_change_id(&local_dir)
            .context("failed to auto-detect change_id from working copy")?,
    };

    let commit_sha = resolve::resolve_commit_sha(&local_dir, change_id)
        .context("failed to resolve change_id to commit SHA")?;

    let repo = git2::Repository::open(&local_dir)
        .with_context(|| format!("failed to open git repository at {}", cli.dir))?;

    let all_ported = get_all_ported_comments(&repo, change_id, commit_sha)
        .map_err(|e| anyhow::anyhow!("failed to read comments: {e}"))?;

    let mut files: Vec<FileComments> = Vec::new();
    let file_filter: Option<PathBuf> = cli.file.map(PathBuf::from);

    for (path, ported_comments) in &all_ported {
        if file_filter.as_ref().is_some_and(|f| f != path) {
            continue;
        }

        let comments: Vec<CommentOutput> = ported_comments
            .iter()
            .filter(|pc| cli.all || !pc.comment.resolved)
            .map(CommentOutput::from)
            .collect();

        if !comments.is_empty() {
            files.push(FileComments {
                path: path.to_string_lossy().to_string(),
                comments,
            });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    let output = Output { files };
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
