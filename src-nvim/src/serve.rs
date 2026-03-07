use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    id: u64,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize)]
struct Response {
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Response {
    fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: u64, error: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

pub fn run(local_dir: &Path) -> Result<()> {
    let repo = git2::Repository::open(local_dir)
        .with_context(|| format!("failed to open git repository at {}", local_dir.display()))?;

    let stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    for line in stdin.lines() {
        let line = line.context("failed to read from stdin")?;
        if line.is_empty() {
            continue;
        }

        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::err(0, format!("invalid request: {e}"));
                write_response(&mut stdout, &resp)?;
                continue;
            }
        };

        let resp = dispatch(&repo, local_dir, &req);
        write_response(&mut stdout, &resp)?;
    }

    Ok(())
}

fn write_response(out: &mut impl Write, resp: &Response) -> Result<()> {
    let json = serde_json::to_string(resp).context("failed to serialize response")?;
    writeln!(out, "{json}")?;
    out.flush()?;
    Ok(())
}

fn dispatch(repo: &git2::Repository, local_dir: &Path, req: &Request) -> Response {
    match req.method.as_str() {
        "log" => handle_log(req.id, local_dir),
        "files" => handle_files(req.id, repo, local_dir, &req.params),
        "blob" => handle_blob(req.id, repo, &req.params),
        "mark-file" => handle_mark(req.id, repo, &req.params),
        "unmark-file" => handle_unmark(req.id, repo, &req.params),
        "set-blob" => handle_set_blob(req.id, repo, &req.params),
        _ => Response::err(req.id, format!("unknown method: {}", req.method)),
    }
}

fn handle_log(id: u64, local_dir: &Path) -> Response {
    match kenjutu_core::services::graph::get_log_graph(local_dir) {
        Ok(graph) => match serde_json::to_value(&graph) {
            Ok(v) => Response::ok(id, v),
            Err(e) => Response::err(id, format!("failed to serialize log: {e}")),
        },
        Err(e) => Response::err(id, format!("failed to get jj log graph: {e}")),
    }
}

#[derive(Deserialize)]
struct FilesParams {
    change_id: ChangeId,
}

fn handle_files(
    id: u64,
    repo: &git2::Repository,
    local_dir: &Path,
    params: &serde_json::Value,
) -> Response {
    let params: FilesParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let commit_id = match find_commit_from_change_id(local_dir, &params.change_id) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to find commit ID: {e:#}")),
    };

    match kenjutu_core::services::diff::generate_file_list(repo, commit_id) {
        Ok((change_id, files)) => {
            let output = serde_json::json!({
                "commitId": commit_id,
                "changeId": change_id,
                "files": files,
            });
            Response::ok(id, output)
        }
        Err(e) => Response::err(id, format!("failed to generate file list: {e}")),
    }
}

#[derive(Deserialize)]
struct BlobParams {
    change_id: ChangeId,
    commit: CommitId,
    file: PathBuf,
    old_path: Option<PathBuf>,
    tree: String,
}

fn handle_blob(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: BlobParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let marker = match MarkerCommit::get(repo, params.change_id, params.commit) {
        Ok(m) => m,
        Err(e) => return Response::err(id, format!("failed to get marker commit: {e}")),
    };

    let tree = match params.tree.as_str() {
        "base" => marker.base_tree(),
        "marker" => marker.marker_tree(),
        "target" => marker.target_tree(),
        other => return Response::err(id, format!("invalid tree kind: {other}")),
    };

    let lookup_path = match params.tree.as_str() {
        "target" => &params.file,
        "base" => params.old_path.as_ref().unwrap_or(&params.file),
        "marker" => &params.file,
        _ => &params.file,
    };

    let content = match tree.get_path(lookup_path) {
        Ok(entry) => match repo.find_blob(entry.id()) {
            Ok(blob) => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(blob.content())
            }
            Err(e) => return Response::err(id, format!("failed to read blob: {e}")),
        },
        Err(_) if params.tree == "marker" => {
            if let Some(ref old_path) = params.old_path {
                match tree.get_path(old_path) {
                    Ok(entry) => match repo.find_blob(entry.id()) {
                        Ok(blob) => {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.encode(blob.content())
                        }
                        Err(e) => return Response::err(id, format!("failed to read blob: {e}")),
                    },
                    Err(_) => String::new(),
                }
            } else {
                String::new()
            }
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => String::new(),
        Err(e) => return Response::err(id, format!("failed to look up file in tree: {e}")),
    };

    Response::ok(id, serde_json::json!({ "content": content }))
}

#[derive(Deserialize)]
struct MarkParams {
    change_id: ChangeId,
    commit: CommitId,
    file: PathBuf,
    old_path: Option<PathBuf>,
}

fn handle_mark(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: MarkParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut marker = match MarkerCommit::get(repo, params.change_id, params.commit) {
        Ok(m) => m,
        Err(e) => return Response::err(id, format!("failed to get marker commit: {e}")),
    };

    if let Err(e) = marker.mark_file_reviewed(&params.file, params.old_path.as_deref()) {
        return Response::err(id, format!("failed to mark file reviewed: {e}"));
    }

    if let Err(e) = marker.write() {
        return Response::err(id, format!("failed to write marker commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

fn handle_unmark(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: MarkParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut marker = match MarkerCommit::get(repo, params.change_id, params.commit) {
        Ok(m) => m,
        Err(e) => return Response::err(id, format!("failed to get marker commit: {e}")),
    };

    if let Err(e) = marker.unmark_file_reviewed(&params.file, params.old_path.as_deref()) {
        return Response::err(id, format!("failed to unmark file reviewed: {e}"));
    }

    if let Err(e) = marker.write() {
        return Response::err(id, format!("failed to write marker commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
struct SetBlobParams {
    change_id: ChangeId,
    commit: CommitId,
    file: PathBuf,
    old_path: Option<PathBuf>,
    content: String,
}

fn handle_set_blob(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: SetBlobParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    use base64::Engine;
    let content = match base64::engine::general_purpose::STANDARD.decode(&params.content) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("invalid base64 content: {e}")),
    };

    let mut marker = match MarkerCommit::get(repo, params.change_id, params.commit) {
        Ok(m) => m,
        Err(e) => return Response::err(id, format!("failed to get marker commit: {e}")),
    };

    if let Err(e) = marker.set_blob(&params.file, params.old_path.as_deref(), &content) {
        return Response::err(id, format!("failed to set blob: {e}"));
    }

    if let Err(e) = marker.write() {
        return Response::err(id, format!("failed to write marker commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

fn find_commit_from_change_id(dir: &Path, change_id: &ChangeId) -> Result<CommitId> {
    let output = Command::new("jj")
        .args([
            "log",
            "-r",
            &change_id.to_string(),
            "-T",
            "commit_id",
            "--no-graph",
        ])
        .current_dir(dir)
        .output()
        .context("failed to execute jj log command")?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let commit_id_str = stdout.trim();
        Ok(commit_id_str.parse()?)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "jj log failed with status {}: {}",
            output.status,
            stderr.trim()
        ))
    }
}
