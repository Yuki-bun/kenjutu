use std::process::Command;

use anyhow::{Context, Result, bail};
use kenjutu_types::{ChangeId, CommitId};

/// Auto-detect the current change_id by running `jj log -r @ -T "change_id"`.
pub fn auto_detect_change_id(local_dir: &str) -> Result<ChangeId> {
    let output = Command::new("jj")
        .args([
            "log",
            "--no-graph",
            "-r",
            "@",
            "-T",
            "change_id",
            "--ignore-working-copy",
        ])
        .current_dir(local_dir)
        .output()
        .context("failed to run jj log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj log failed: {}", stderr.trim());
    }

    let raw = String::from_utf8(output.stdout).context("jj output is not valid UTF-8")?;
    let id_str = raw.trim();

    ChangeId::try_from(id_str).map_err(|e| anyhow::anyhow!("invalid change_id from jj: {e}"))
}

/// Resolve a change_id to a commit SHA via `jj log --no-graph -r <change_id> -T "commit_id"`.
pub fn resolve_commit_sha(local_dir: &str, change_id: ChangeId) -> Result<CommitId> {
    let output = Command::new("jj")
        .args([
            "log",
            "--no-graph",
            "-r",
            &change_id.to_string(),
            "-T",
            "commit_id",
            "--ignore-working-copy",
        ])
        .current_dir(local_dir)
        .output()
        .context("failed to run jj log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj log failed: {}", stderr.trim());
    }

    let raw = String::from_utf8(output.stdout).context("jj output is not valid UTF-8")?;
    let sha_str = raw.trim();

    sha_str
        .parse::<CommitId>()
        .map_err(|e| anyhow::anyhow!("invalid commit SHA from jj: {e}"))
}
