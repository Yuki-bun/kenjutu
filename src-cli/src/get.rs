use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use comment_commit::get_all_ported_comments;
use kenjutu_types::ChangeId;
use serde::Serialize;

use crate::resolve;

#[derive(Debug, Serialize)]
struct Output {
    files: Vec<FileComments>,
}

#[derive(Debug, Serialize)]
struct FileComments {
    path: String,
    comments: Vec<CommentOutput>,
}

#[derive(Debug, Serialize)]
struct CommentOutput {
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    side: String,
    body: String,
    target_sha: String,
    resolved: bool,
    context: ContextOutput,
    replies: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ContextOutput {
    before: String,
    target: String,
    after: String,
}

impl From<&comment_commit::AnchorContext> for ContextOutput {
    fn from(anchor: &comment_commit::AnchorContext) -> Self {
        Self {
            before: anchor.before.join("\n"),
            target: anchor.target.join("\n"),
            after: anchor.after.join("\n"),
        }
    }
}

impl From<&comment_commit::PortedComment> for CommentOutput {
    fn from(pc: &comment_commit::PortedComment) -> Self {
        let c = &pc.comment;
        Self {
            line: pc.ported_line,
            start_line: pc.ported_start_line,
            side: match c.side {
                comment_commit::DiffSide::Old => "old".to_string(),
                comment_commit::DiffSide::New => "new".to_string(),
            },
            body: c.body.clone(),
            target_sha: c.target_sha.to_string(),
            resolved: c.resolved,
            context: ContextOutput::from(&c.anchor),
            replies: c.replies.iter().map(|r| r.body.clone()).collect(),
        }
    }
}

pub fn run(
    local_dir: &Path,
    dir: &str,
    change_id: Option<String>,
    file: Option<String>,
    all: bool,
) -> Result<()> {
    let change_id: ChangeId = match change_id {
        Some(raw) => raw
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid --change-id: {e}"))?,
        None => resolve::auto_detect_change_id(local_dir)
            .context("failed to auto-detect change_id from working copy")?,
    };

    let commit_sha = resolve::resolve_commit_sha(local_dir, change_id)
        .context("failed to resolve change_id to commit SHA")?;

    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", dir))?;

    let all_ported = get_all_ported_comments(&repo, commit_sha)
        .map_err(|e| anyhow::anyhow!("failed to read comments: {e}"))?;

    let mut files: Vec<FileComments> = Vec::new();
    let file_filter: Option<PathBuf> = file.map(PathBuf::from);

    for (path, ported_comments) in &all_ported {
        if file_filter.as_ref().is_some_and(|f| f != path) {
            continue;
        }

        let comments: Vec<CommentOutput> = ported_comments
            .iter()
            .filter(|pc| all || !pc.comment.resolved)
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
