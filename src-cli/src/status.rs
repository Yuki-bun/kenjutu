use std::path::Path;

use anyhow::{Context, Result};
use comment_commit::CommentCommit;
use serde::Serialize;

use crate::resolve;

#[derive(Debug, Serialize)]
struct StatusEntry {
    change_id: String,
    description: String,
    unresolved: u64,
    resolved: u64,
}

pub fn run(local_dir: &Path, revset: Option<String>, all: bool) -> Result<()> {
    let revset_entries = resolve::resolve_revset(local_dir, revset.as_deref())
        .context("failed to resolve revset")?;

    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let mut entries: Vec<StatusEntry> = Vec::new();

    for re in revset_entries {
        let cc = CommentCommit::get(&repo, re.change_id)
            .map_err(|e| anyhow::anyhow!("failed to read comments for {}: {e}", re.change_id))?;

        let all_comments = cc.get_all_comments();
        let (resolved, unresolved) =
            all_comments
                .values()
                .flatten()
                .fold((0, 0), |(res, un_res), comment| {
                    if comment.resolved {
                        (res + 1, un_res)
                    } else {
                        (res, un_res + 1)
                    }
                });

        if !all && unresolved != 0 {
            entries.push(StatusEntry {
                change_id: re.change_id.to_string(),
                description: re.description,
                unresolved,
                resolved,
            });
        }
    }

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
