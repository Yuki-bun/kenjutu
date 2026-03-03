use serde::Serialize;

/// Top-level output: a list of files with their comments.
#[derive(Debug, Serialize)]
pub struct Output {
    pub files: Vec<FileComments>,
}

/// All comments for a single file.
#[derive(Debug, Serialize)]
pub struct FileComments {
    pub path: String,
    pub comments: Vec<CommentOutput>,
}

/// A single comment with context, ported line info, and replies.
#[derive(Debug, Serialize)]
pub struct CommentOutput {
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    pub side: String,
    pub body: String,
    pub target_sha: String,
    pub resolved: bool,
    pub context: ContextOutput,
    pub replies: Vec<String>,
}

/// The anchor lines: up to 3 before, the commented line(s), up to 3 after.
/// Each field is a single string with lines joined by newlines.
#[derive(Debug, Serialize)]
pub struct ContextOutput {
    pub before: String,
    pub target: String,
    pub after: String,
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
