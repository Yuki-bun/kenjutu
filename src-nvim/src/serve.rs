use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use comment_commit::{CommentCommit, DiffSide, get_all_ported_comments};
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
        "files" => handle_files(req.id, repo, local_dir, &req.params),
        "blob" => handle_blob(req.id, repo, &req.params),
        "mark-file" => handle_mark(req.id, repo, &req.params),
        "unmark-file" => handle_unmark(req.id, repo, &req.params),
        "set-blob" => handle_set_blob(req.id, repo, &req.params),
        "get-comments" => handle_get_comments(req.id, repo, &req.params),
        "add-comment" => handle_add_comment(req.id, repo, &req.params),
        "reply-to-comment" => handle_reply_to_comment(req.id, repo, &req.params),
        "edit-comment" => handle_edit_comment(req.id, repo, &req.params),
        "resolve-comment" => handle_resolve_comment(req.id, repo, &req.params),
        "unresolve-comment" => handle_unresolve_comment(req.id, repo, &req.params),
        _ => Response::err(req.id, format!("unknown method: {}", req.method)),
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
    commit: CommitId,
    file: PathBuf,
    old_path: Option<PathBuf>,
    tree: String,
}

fn blob_to_string(id: u64, blob: &git2::Blob) -> Result<String, Response> {
    if blob.is_binary() {
        return Err(Response::err(id, "file is binary".to_owned()));
    }
    std::str::from_utf8(blob.content())
        .map(|s| s.to_owned())
        .map_err(|_| Response::err(id, "invalid content: not valid UTF-8".to_owned()))
}

macro_rules! try_or_return {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(resp) => return resp,
        }
    };
}

fn handle_blob(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: BlobParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let marker = match MarkerCommit::get(repo, params.commit) {
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
            Ok(blob) => try_or_return!(blob_to_string(id, &blob)),
            Err(e) => return Response::err(id, format!("failed to read blob: {e}")),
        },
        Err(_) if params.tree == "marker" => {
            if let Some(ref old_path) = params.old_path {
                match tree.get_path(old_path) {
                    Ok(entry) => match repo.find_blob(entry.id()) {
                        Ok(blob) => try_or_return!(blob_to_string(id, &blob)),
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
    commit: CommitId,
    file: PathBuf,
    old_path: Option<PathBuf>,
}

fn handle_mark(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: MarkParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut marker = match MarkerCommit::get(repo, params.commit) {
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

    let mut marker = match MarkerCommit::get(repo, params.commit) {
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

    let content = params.content.as_bytes();

    let mut marker = match MarkerCommit::get(repo, params.commit) {
        Ok(m) => m,
        Err(e) => return Response::err(id, format!("failed to get marker commit: {e}")),
    };

    if let Err(e) = marker.set_blob(&params.file, params.old_path.as_deref(), content) {
        return Response::err(id, format!("failed to set blob: {e}"));
    }

    if let Err(e) = marker.write() {
        return Response::err(id, format!("failed to write marker commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
struct GetCommentsParams {
    commit: CommitId,
}

fn handle_get_comments(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: GetCommentsParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let ported = match get_all_ported_comments(repo, params.commit) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("failed to get comments: {e}")),
    };

    let mut files: Vec<serde_json::Value> = ported
        .into_iter()
        .map(|(path, comments)| {
            serde_json::json!({
                "file_path": path.to_string_lossy(),
                "comments": comments,
            })
        })
        .collect();

    files.sort_by(|a, b| {
        let a_path = a["file_path"].as_str().unwrap_or("");
        let b_path = b["file_path"].as_str().unwrap_or("");
        a_path.cmp(b_path)
    });

    Response::ok(id, serde_json::json!({ "files": files }))
}

#[derive(Deserialize)]
struct AddCommentParams {
    commit: CommitId,
    file: PathBuf,
    side: DiffSide,
    line: u32,
    start_line: Option<u32>,
    body: String,
}

fn handle_add_comment(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: AddCommentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut cc = match CommentCommit::get(repo, params.commit) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to get comment commit: {e}")),
    };

    if let Err(e) = cc.create_comment(
        params.commit,
        &params.file,
        params.side,
        params.line,
        params.start_line,
        params.body,
    ) {
        return Response::err(id, format!("failed to create comment: {e}"));
    }

    if let Err(e) = cc.write() {
        return Response::err(id, format!("failed to write comment commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
struct ReplyToCommentParams {
    file: PathBuf,
    commit: CommitId,
    parent_comment_id: String,
    body: String,
}

fn handle_reply_to_comment(
    id: u64,
    repo: &git2::Repository,
    params: &serde_json::Value,
) -> Response {
    let params: ReplyToCommentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut cc = match CommentCommit::get(repo, params.commit) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to get comment commit: {e}")),
    };

    if let Err(e) = cc.reply_to_comment(&params.file, params.parent_comment_id, params.body) {
        return Response::err(id, format!("failed to reply to comment: {e}"));
    }

    if let Err(e) = cc.write() {
        return Response::err(id, format!("failed to write comment commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
struct EditCommentParams {
    file: PathBuf,
    commit: CommitId,
    comment_id: String,
    body: String,
}

fn handle_edit_comment(id: u64, repo: &git2::Repository, params: &serde_json::Value) -> Response {
    let params: EditCommentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut cc = match CommentCommit::get(repo, params.commit) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to get comment commit: {e}")),
    };

    if let Err(e) = cc.edit_comment(&params.file, params.comment_id, params.body) {
        return Response::err(id, format!("failed to edit comment: {e}"));
    }

    if let Err(e) = cc.write() {
        return Response::err(id, format!("failed to write comment commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
struct ResolveCommentParams {
    file: PathBuf,
    comment_id: String,
    commit: CommitId,
}

fn handle_resolve_comment(
    id: u64,
    repo: &git2::Repository,
    params: &serde_json::Value,
) -> Response {
    let params: ResolveCommentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut cc = match CommentCommit::get(repo, params.commit) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to get comment commit: {e}")),
    };

    if let Err(e) = cc.resolve_comment(&params.file, params.comment_id) {
        return Response::err(id, format!("failed to resolve comment: {e}"));
    }

    if let Err(e) = cc.write() {
        return Response::err(id, format!("failed to write comment commit: {e}"));
    }

    Response::ok(id, serde_json::json!({ "success": true }))
}

fn handle_unresolve_comment(
    id: u64,
    repo: &git2::Repository,
    params: &serde_json::Value,
) -> Response {
    let params: ResolveCommentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return Response::err(id, format!("invalid params: {e}")),
    };

    let mut cc = match CommentCommit::get(repo, params.commit) {
        Ok(c) => c,
        Err(e) => return Response::err(id, format!("failed to get comment commit: {e}")),
    };

    if let Err(e) = cc.unresolve_comment(&params.file, params.comment_id) {
        return Response::err(id, format!("failed to unresolve comment: {e}"));
    }

    if let Err(e) = cc.write() {
        return Response::err(id, format!("failed to write comment commit: {e}"));
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
