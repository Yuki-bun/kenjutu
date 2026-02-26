use std::collections::HashMap;

use crate::model::{ActionEntry, CommentAction, MaterializedComment, MaterializedReply};

/// Replay an action log to produce the current state of all comment threads.
///
/// Actions are sorted by `created_at` (stable sort) before replay, so callers
/// do not need to guarantee ordering. Equal timestamps preserve their original
/// array order, which is correct for the append-only single-user case.
///
/// Actions with unknown `comment_id` references are silently skipped for robustness
/// (e.g. partial sync scenarios where actions arrive out of order).
pub(crate) fn materialize(actions: &[ActionEntry]) -> Vec<MaterializedComment> {
    // Sort by timestamp. Stable sort preserves original order for equal timestamps.
    let mut sorted: Vec<&ActionEntry> = actions.iter().collect();
    sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let mut comments: HashMap<String, MaterializedComment> = HashMap::new();
    // Track insertion order so output is deterministic.
    let mut order: Vec<String> = Vec::new();
    // Map reply IDs to their parent comment ID for Edit lookups.
    let mut reply_parent: HashMap<String, String> = HashMap::new();

    for entry in &sorted {
        let timestamp = &entry.created_at;
        match &entry.action {
            CommentAction::Create {
                comment_id,
                target_sha,
                side,
                line,
                start_line,
                body,
                anchor,
            } => {
                if comments.contains_key(comment_id) {
                    // Duplicate Create — skip.
                    continue;
                }
                order.push(comment_id.clone());
                comments.insert(
                    comment_id.clone(),
                    MaterializedComment {
                        id: comment_id.clone(),
                        target_sha: *target_sha,
                        side: *side,
                        line: *line,
                        start_line: *start_line,
                        body: body.clone(),
                        anchor: anchor.clone(),
                        resolved: false,
                        created_at: timestamp.clone(),
                        updated_at: timestamp.clone(),
                        edit_count: 0,
                        replies: Vec::new(),
                    },
                );
            }
            CommentAction::Reply {
                comment_id,
                parent_comment_id,
                body,
            } => {
                if let Some(parent) = comments.get_mut(parent_comment_id) {
                    reply_parent.insert(comment_id.clone(), parent_comment_id.clone());
                    parent.replies.push(MaterializedReply {
                        id: comment_id.clone(),
                        body: body.clone(),
                        created_at: timestamp.clone(),
                        updated_at: timestamp.clone(),
                        edit_count: 0,
                    });
                    parent.updated_at = timestamp.clone();
                }
                // If parent doesn't exist, silently skip.
            }
            CommentAction::Edit { comment_id, body } => {
                // Check if it's a top-level comment.
                if let Some(comment) = comments.get_mut(comment_id) {
                    comment.body = body.clone();
                    comment.updated_at = timestamp.clone();
                    comment.edit_count += 1;
                } else if let Some(parent_id) = reply_parent.get(comment_id) {
                    // It's a reply — find it in the parent's replies.
                    if let Some(parent) = comments.get_mut(parent_id) {
                        if let Some(reply) = parent.replies.iter_mut().find(|r| r.id == *comment_id)
                        {
                            reply.body = body.clone();
                            reply.updated_at = timestamp.clone();
                            reply.edit_count += 1;
                        }
                        parent.updated_at = timestamp.clone();
                    }
                }
                // Unknown comment_id — silently skip.
            }
            CommentAction::Resolve { comment_id } => {
                if let Some(comment) = comments.get_mut(comment_id) {
                    comment.resolved = true;
                    comment.updated_at = timestamp.clone();
                }
            }
            CommentAction::Unresolve { comment_id } => {
                if let Some(comment) = comments.get_mut(comment_id) {
                    comment.resolved = false;
                    comment.updated_at = timestamp.clone();
                }
            }
        }
    }

    // Return in insertion order.
    order
        .into_iter()
        .filter_map(|id| comments.remove(&id))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::CommitId;
    use crate::model::{AnchorContext, CommentAction, DiffSide};

    use super::*;

    fn dummy_sha() -> CommitId {
        "0000000000000000000000000000000000000000".parse().unwrap()
    }

    fn make_anchor() -> AnchorContext {
        AnchorContext {
            before: vec!["line before".to_string()],
            target: vec!["target line".to_string()],
            after: vec!["line after".to_string()],
        }
    }

    fn action(action_id: &str, created_at: &str, action: CommentAction) -> ActionEntry {
        ActionEntry {
            action_id: action_id.to_string(),
            created_at: created_at.to_string(),
            action,
        }
    }

    #[test]
    fn test_create_single_comment() {
        let actions = vec![action(
            "act-1",
            "2025-01-01T00:00:00Z",
            CommentAction::Create {
                comment_id: "c1".to_string(),
                target_sha: dummy_sha(),
                side: DiffSide::New,
                line: 42,
                start_line: None,
                body: "looks wrong".to_string(),
                anchor: make_anchor(),
            },
        )];

        let result = materialize(&actions);
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.id, "c1");
        assert_eq!(c.side, DiffSide::New);
        assert_eq!(c.line, 42);
        assert_eq!(c.body, "looks wrong");
        assert!(!c.resolved);
        assert_eq!(c.edit_count, 0);
        assert!(c.replies.is_empty());
    }

    #[test]
    fn test_reply_to_comment() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 10,
                    start_line: None,
                    body: "question".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Reply {
                    comment_id: "r1".to_string(),
                    parent_comment_id: "c1".to_string(),
                    body: "answer".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].replies.len(), 1);
        assert_eq!(result[0].replies[0].id, "r1");
        assert_eq!(result[0].replies[0].body, "answer");
        assert_eq!(result[0].updated_at, "2025-01-01T00:01:00Z");
    }

    #[test]
    fn test_edit_top_level_comment() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::Old,
                    line: 5,
                    start_line: None,
                    body: "original".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:05:00Z",
                CommentAction::Edit {
                    comment_id: "c1".to_string(),
                    body: "edited".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result[0].body, "edited");
        assert_eq!(result[0].edit_count, 1);
        assert_eq!(result[0].updated_at, "2025-01-01T00:05:00Z");
    }

    #[test]
    fn test_edit_reply() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 10,
                    start_line: None,
                    body: "question".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Reply {
                    comment_id: "r1".to_string(),
                    parent_comment_id: "c1".to_string(),
                    body: "first answer".to_string(),
                },
            ),
            action(
                "act-3",
                "2025-01-01T00:02:00Z",
                CommentAction::Edit {
                    comment_id: "r1".to_string(),
                    body: "corrected answer".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result[0].replies[0].body, "corrected answer");
        assert_eq!(result[0].replies[0].edit_count, 1);
        assert_eq!(result[0].updated_at, "2025-01-01T00:02:00Z");
    }

    #[test]
    fn test_resolve_and_unresolve() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 1,
                    start_line: None,
                    body: "fix this".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:10:00Z",
                CommentAction::Resolve {
                    comment_id: "c1".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert!(result[0].resolved);

        // Now unresolve.
        let mut actions = actions;
        actions.push(action(
            "act-3",
            "2025-01-01T00:20:00Z",
            CommentAction::Unresolve {
                comment_id: "c1".to_string(),
            },
        ));

        let result = materialize(&actions);
        assert!(!result[0].resolved);
        assert_eq!(result[0].updated_at, "2025-01-01T00:20:00Z");
    }

    #[test]
    fn test_unknown_comment_id_is_skipped() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Edit {
                    comment_id: "nonexistent".to_string(),
                    body: "this goes nowhere".to_string(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:00:00Z",
                CommentAction::Resolve {
                    comment_id: "nonexistent".to_string(),
                },
            ),
            action(
                "act-3",
                "2025-01-01T00:00:00Z",
                CommentAction::Reply {
                    comment_id: "r1".to_string(),
                    parent_comment_id: "nonexistent".to_string(),
                    body: "orphan reply".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert!(result.is_empty());
    }

    #[test]
    fn test_duplicate_create_is_skipped() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 1,
                    start_line: None,
                    body: "first".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::Old,
                    line: 99,
                    start_line: None,
                    body: "duplicate".to_string(),
                    anchor: make_anchor(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body, "first");
        assert_eq!(result[0].line, 1);
    }

    #[test]
    fn test_multiple_comments_preserve_order() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 10,
                    start_line: None,
                    body: "first comment".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Create {
                    comment_id: "c2".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::Old,
                    line: 20,
                    start_line: None,
                    body: "second comment".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-3",
                "2025-01-01T00:02:00Z",
                CommentAction::Create {
                    comment_id: "c3".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 30,
                    start_line: None,
                    body: "third comment".to_string(),
                    anchor: make_anchor(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "c1");
        assert_eq!(result[1].id, "c2");
        assert_eq!(result[2].id, "c3");
    }

    #[test]
    fn test_multiline_comment() {
        let actions = vec![action(
            "act-1",
            "2025-01-01T00:00:00Z",
            CommentAction::Create {
                comment_id: "c1".to_string(),
                target_sha: dummy_sha(),
                side: DiffSide::New,
                line: 15,
                start_line: Some(10),
                body: "this whole block is wrong".to_string(),
                anchor: make_anchor(),
            },
        )];

        let result = materialize(&actions);
        assert_eq!(result[0].start_line, Some(10));
        assert_eq!(result[0].line, 15);
    }

    #[test]
    fn test_multiple_replies() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 1,
                    start_line: None,
                    body: "question".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Reply {
                    comment_id: "r1".to_string(),
                    parent_comment_id: "c1".to_string(),
                    body: "reply 1".to_string(),
                },
            ),
            action(
                "act-3",
                "2025-01-01T00:02:00Z",
                CommentAction::Reply {
                    comment_id: "r2".to_string(),
                    parent_comment_id: "c1".to_string(),
                    body: "reply 2".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result[0].replies.len(), 2);
        assert_eq!(result[0].replies[0].id, "r1");
        assert_eq!(result[0].replies[1].id, "r2");
    }

    #[test]
    fn test_multiple_edits() {
        let actions = vec![
            action(
                "act-1",
                "2025-01-01T00:00:00Z",
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    target_sha: dummy_sha(),
                    side: DiffSide::New,
                    line: 1,
                    start_line: None,
                    body: "v1".to_string(),
                    anchor: make_anchor(),
                },
            ),
            action(
                "act-2",
                "2025-01-01T00:01:00Z",
                CommentAction::Edit {
                    comment_id: "c1".to_string(),
                    body: "v2".to_string(),
                },
            ),
            action(
                "act-3",
                "2025-01-01T00:02:00Z",
                CommentAction::Edit {
                    comment_id: "c1".to_string(),
                    body: "v3".to_string(),
                },
            ),
        ];

        let result = materialize(&actions);
        assert_eq!(result[0].body, "v3");
        assert_eq!(result[0].edit_count, 2);
    }
}
