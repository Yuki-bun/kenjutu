use std::path::PathBuf;

use comment_commit::{get_all_ported_comments, CommentCommit, DiffSide, PortedComment};
use kenjutu_types::{ChangeId, CommitId};
use serde::Deserialize;
use specta::Type;
use tauri::command;

use super::{Error, Result};
use kenjutu_core::services::git;

// --- Input structs ---

#[derive(Deserialize, Type)]
pub struct AddCommentInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub sha: CommitId,
    pub file_path: String,
    pub side: DiffSide,
    pub line: u32,
    pub start_line: Option<u32>,
    pub body: String,
}

#[derive(Deserialize, Type)]
pub struct ReplyToCommentInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub file_path: String,
    pub parent_comment_id: String,
    pub body: String,
}

#[derive(Deserialize, Type)]
pub struct EditCommentInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub file_path: String,
    pub comment_id: String,
    pub body: String,
}

#[derive(Deserialize, Type)]
pub struct ResolveCommentInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub file_path: String,
    pub comment_id: String,
}

#[derive(Deserialize, Type)]
pub struct UnresolveCommentInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub file_path: String,
    pub comment_id: String,
}

#[derive(Deserialize, Type)]
pub struct GetCommentsInput {
    pub local_dir: String,
    pub change_id: ChangeId,
    pub sha: CommitId,
}

// --- Return types ---

#[derive(serde::Serialize, Type)]
pub struct FileComments {
    pub file_path: String,
    pub comments: Vec<PortedComment>,
}

// --- Commands ---

#[command]
#[specta::specta]
pub async fn add_comment(input: AddCommentInput) -> Result<()> {
    let repo = git::open_repository(&input.local_dir)?;
    let mut cc = CommentCommit::get(&repo, input.change_id).map_err(map_comment_err)?;

    let file_path = PathBuf::from(&input.file_path);

    cc.create_comment(
        input.sha,
        &file_path,
        input.side,
        input.line,
        input.start_line,
        input.body,
    )
    .map_err(map_comment_err)?;

    cc.write().map_err(map_comment_err)?;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn reply_to_comment(input: ReplyToCommentInput) -> Result<()> {
    let repo = git::open_repository(&input.local_dir)?;
    let mut cc = CommentCommit::get(&repo, input.change_id).map_err(map_comment_err)?;

    let file_path = PathBuf::from(&input.file_path);

    cc.reply_to_comment(&file_path, input.parent_comment_id, input.body)
        .map_err(map_comment_err)?;

    cc.write().map_err(map_comment_err)?;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn edit_comment(input: EditCommentInput) -> Result<()> {
    let repo = git::open_repository(&input.local_dir)?;
    let mut cc = CommentCommit::get(&repo, input.change_id).map_err(map_comment_err)?;

    let file_path = PathBuf::from(&input.file_path);

    cc.edit_comment(&file_path, input.comment_id, input.body)
        .map_err(map_comment_err)?;

    cc.write().map_err(map_comment_err)?;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn resolve_comment(input: ResolveCommentInput) -> Result<()> {
    let repo = git::open_repository(&input.local_dir)?;
    let mut cc = CommentCommit::get(&repo, input.change_id).map_err(map_comment_err)?;

    let file_path = PathBuf::from(&input.file_path);

    cc.resolve_comment(&file_path, input.comment_id)
        .map_err(map_comment_err)?;

    cc.write().map_err(map_comment_err)?;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn unresolve_comment(input: UnresolveCommentInput) -> Result<()> {
    let repo = git::open_repository(&input.local_dir)?;
    let mut cc = CommentCommit::get(&repo, input.change_id).map_err(map_comment_err)?;

    let file_path = PathBuf::from(&input.file_path);

    cc.unresolve_comment(&file_path, input.comment_id)
        .map_err(map_comment_err)?;

    cc.write().map_err(map_comment_err)?;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn get_comments(input: GetCommentsInput) -> Result<Vec<FileComments>> {
    let repo = git::open_repository(&input.local_dir)?;
    let ported =
        get_all_ported_comments(&repo, input.change_id, input.sha).map_err(map_comment_err)?;

    let mut result: Vec<FileComments> = ported
        .into_iter()
        .map(|(path, comments)| FileComments {
            file_path: path.to_string_lossy().to_string(),
            comments,
        })
        .collect();

    // Sort by file path for deterministic output.
    result.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    Ok(result)
}

// --- Helpers ---

fn map_comment_err(err: comment_commit::Error) -> Error {
    Error::CommentCommit {
        message: err.to_string(),
    }
}
