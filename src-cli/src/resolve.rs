use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use kenjutu_types::{ChangeId, CommitId};

pub struct RevsetEntry {
    pub change_id: ChangeId,
    pub description: String,
}

pub fn resolve_revset(local_dir: &Path, revset: Option<&str>) -> Result<Vec<RevsetEntry>> {
    let mut cmd = Command::new("jj");
    cmd.args(["log", "--no-graph"]);
    if let Some(r) = revset {
        cmd.args(["-r", r]);
    }
    cmd.args([
        "-T",
        r#"change_id ++ "\t" ++ description.first_line() ++ "\n""#,
        "--ignore-working-copy",
    ]);
    cmd.current_dir(local_dir);

    let output = cmd.output().context("failed to run jj log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj log failed: {}", stderr.trim());
    }

    let raw = String::from_utf8(output.stdout).context("jj output is not valid UTF-8")?;
    raw.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (id_str, desc) = line
                .split_once('\t')
                .ok_or_else(|| anyhow::anyhow!("unexpected jj output format: {line}"))?;
            let change_id: ChangeId = id_str
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid change_id from jj: {e}"))?;
            Ok(RevsetEntry {
                change_id,
                description: desc.to_string(),
            })
        })
        .collect()
}

/// Auto-detect the current change_id by running `jj log -r @ -T "change_id"`.
pub fn auto_detect_change_id(local_dir: &Path) -> Result<ChangeId> {
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

    id_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid change_id from jj: {e}"))
}

/// Resolve a change_id to a commit SHA via `jj log --no-graph -r <change_id> -T "commit_id"`.
pub fn resolve_commit_sha(local_dir: &Path, change_id: ChangeId) -> Result<CommitId> {
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
