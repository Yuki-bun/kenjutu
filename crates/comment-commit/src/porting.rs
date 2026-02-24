use std::collections::HashMap;
use std::path::{Path, PathBuf};

use git2::Repository;

use crate::comment_commit::{CommentCommit, enumerate_comment_refs};
use crate::model::{AnchorContext, MaterializedComment, PortedComment};
use crate::{ChangeId, CommitId, Result};

/// Get all comments for a change_id, ported to the current commit SHA.
///
/// This is the main read API for comments. It:
/// 1. Enumerates all `refs/kenjutu/{change_id}/comments/*` refs
/// 2. For the ref matching `current_sha` — returns comments as-is
/// 3. For refs with different SHAs — ports comments using anchor text matching
///
/// Returns a map of file_path → ported comments.
pub fn get_all_ported_comments(
    repo: &Repository,
    change_id: ChangeId,
    current_sha: CommitId,
) -> Result<HashMap<PathBuf, Vec<PortedComment>>> {
    let refs = enumerate_comment_refs(repo, change_id)?;
    let mut result: HashMap<PathBuf, Vec<PortedComment>> = HashMap::new();

    // Load file content from the current commit for anchor matching.
    let current_commit = repo.find_commit(current_sha.oid())?;
    let current_tree = current_commit.tree()?;

    for (ref_sha, _ref_name) in &refs {
        let is_current = *ref_sha == current_sha;

        let cc = CommentCommit::get(repo, change_id, *ref_sha)?;
        let all_comments = cc.get_all_comments();

        for (file_path, comments) in all_comments {
            let ported: Vec<PortedComment> = if is_current {
                // Comments on the current SHA — no porting needed.
                comments
                    .into_iter()
                    .map(|c| PortedComment {
                        ported_line: Some(c.line),
                        ported_start_line: c.start_line,
                        is_ported: false,
                        comment: c,
                    })
                    .collect()
            } else {
                // Comments on an older SHA — port using anchor text.
                let file_content = read_file_from_tree(repo, &current_tree, &file_path);
                comments
                    .into_iter()
                    .map(|c| port_comment(c, file_content.as_deref()))
                    .collect()
            };

            result.entry(file_path).or_default().extend(ported);
        }
    }

    Ok(result)
}

/// Port a single comment to a new file content using anchor text matching.
fn port_comment(comment: MaterializedComment, file_content: Option<&str>) -> PortedComment {
    let Some(content) = file_content else {
        // File doesn't exist in current commit — degrade to file-level.
        return PortedComment {
            ported_line: None,
            ported_start_line: None,
            is_ported: true,
            comment,
        };
    };

    let anchor_start = find_anchor_position(content, &comment.anchor);

    // find_anchor_position returns where the target block starts (1-based).
    // For single-line comments, ported_line = anchor_start.
    // For multi-line comments, we need to compute both start and end:
    //   ported_start_line = anchor_start
    //   ported_line = anchor_start + (line - start_line)
    let (ported_line, ported_start_line) = match (anchor_start, comment.start_line) {
        (Some(anchor), Some(start)) => {
            let offset = comment.line.saturating_sub(start);
            (Some(anchor + offset), Some(anchor))
        }
        (Some(anchor), None) => (Some(anchor), None),
        (None, _) => (None, None),
    };

    PortedComment {
        ported_line,
        ported_start_line,
        is_ported: true,
        comment,
    }
}

/// Search for the anchor's target lines in the file content.
///
/// Strategy:
/// 1. Find all positions where the target lines appear in the file
/// 2. If exactly one match — use it
/// 3. If multiple matches — use context (before/after) to disambiguate
/// 4. If no match — return None (comment degrades to file-level)
///
/// Returns the 1-based line number where the target starts.
pub fn find_anchor_position(file_content: &str, anchor: &AnchorContext) -> Option<u32> {
    if anchor.target.is_empty() {
        return None;
    }

    let file_lines: Vec<&str> = file_content.lines().collect();
    if file_lines.is_empty() {
        return None;
    }

    let target_len = anchor.target.len();

    // Find all positions where the target lines match.
    let mut candidates: Vec<usize> = Vec::new();
    for i in 0..=file_lines.len().saturating_sub(target_len) {
        if matches_target(&file_lines[i..i + target_len], &anchor.target) {
            candidates.push(i);
        }
    }

    match candidates.len() {
        0 => None,
        1 => Some(candidates[0] as u32 + 1), // 1-based
        _ => {
            // Multiple matches — use context to disambiguate.
            disambiguate_with_context(&file_lines, &candidates, anchor)
        }
    }
}

/// Check if a slice of file lines matches the target lines.
fn matches_target(file_slice: &[&str], target: &[String]) -> bool {
    if file_slice.len() != target.len() {
        return false;
    }
    file_slice
        .iter()
        .zip(target.iter())
        .all(|(file_line, target_line)| *file_line == target_line.as_str())
}

/// When multiple target matches exist, use before/after context to pick the best one.
fn disambiguate_with_context(
    file_lines: &[&str],
    candidates: &[usize],
    anchor: &AnchorContext,
) -> Option<u32> {
    let target_len = anchor.target.len();
    let mut best_idx = None;
    let mut best_score = 0;

    for &candidate in candidates {
        let mut score = 0;

        // Score before-context matches.
        for (i, before_line) in anchor.before.iter().rev().enumerate() {
            let line_idx = candidate.checked_sub(i + 1);
            if let Some(idx) = line_idx
                && idx < file_lines.len()
                && file_lines[idx] == before_line.as_str()
            {
                score += 1;
            }
        }

        // Score after-context matches.
        for (i, after_line) in anchor.after.iter().enumerate() {
            let line_idx = candidate + target_len + i;
            if line_idx < file_lines.len() && file_lines[line_idx] == after_line.as_str() {
                score += 1;
            }
        }

        if score > best_score {
            best_score = score;
            best_idx = Some(candidate);
        }
    }

    // If no context matched at all, return the first candidate as a best guess.
    let idx = best_idx.unwrap_or(candidates[0]);
    Some(idx as u32 + 1) // 1-based
}

/// Read a file's content from a git tree, returning None if the file doesn't exist.
fn read_file_from_tree(
    repo: &Repository,
    tree: &git2::Tree<'_>,
    file_path: &Path,
) -> Option<String> {
    let entry = tree.get_path(file_path).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    std::str::from_utf8(blob.content()).ok().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AnchorContext, CommentAction, DiffSide};
    use test_repo::TestRepo;

    fn make_anchor(before: &[&str], target: &[&str], after: &[&str]) -> AnchorContext {
        AnchorContext {
            before: before.iter().map(|s| s.to_string()).collect(),
            target: target.iter().map(|s| s.to_string()).collect(),
            after: after.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_find_anchor_exact_match() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let anchor = make_anchor(&["line 2"], &["line 3"], &["line 4"]);
        assert_eq!(find_anchor_position(content, &anchor), Some(3));
    }

    #[test]
    fn test_find_anchor_no_match() {
        let content = "line 1\nline 2\nline 3";
        let anchor = make_anchor(&[], &["nonexistent"], &[]);
        assert_eq!(find_anchor_position(content, &anchor), None);
    }

    #[test]
    fn test_find_anchor_disambiguate_with_context() {
        // "target" appears at lines 2 and 5, but context matches only at line 5.
        let content = "aaa\ntarget\nbbb\nccc\ntarget\nddd";
        let anchor = make_anchor(&["ccc"], &["target"], &["ddd"]);
        assert_eq!(find_anchor_position(content, &anchor), Some(5));
    }

    #[test]
    fn test_find_anchor_multiline_target() {
        let content = "a\nb\nc\nd\ne";
        let anchor = make_anchor(&["a"], &["b", "c"], &["d"]);
        assert_eq!(find_anchor_position(content, &anchor), Some(2));
    }

    #[test]
    fn test_find_anchor_empty_target() {
        let content = "line 1\nline 2";
        let anchor = make_anchor(&[], &[], &[]);
        assert_eq!(find_anchor_position(content, &anchor), None);
    }

    #[test]
    fn test_find_anchor_at_start_of_file() {
        let content = "target\nline 2\nline 3";
        let anchor = make_anchor(&[], &["target"], &["line 2"]);
        assert_eq!(find_anchor_position(content, &anchor), Some(1));
    }

    #[test]
    fn test_find_anchor_at_end_of_file() {
        let content = "line 1\nline 2\ntarget";
        let anchor = make_anchor(&["line 2"], &["target"], &[]);
        assert_eq!(find_anchor_position(content, &anchor), Some(3));
    }

    #[test]
    fn test_port_comments_same_sha() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("main.rs", "fn main() {\n    println!(\"hello\");\n}\n")
            .unwrap();
        let r = test_repo.commit("init").unwrap();
        let sha = r.created.commit_id;
        let change_id = r.created.change_id;

        // Add a comment on the current sha.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 2,
                    start_line: None,
                    body: "nice print".to_string(),
                    anchor: make_anchor(&["fn main() {"], &["    println!(\"hello\");"], &["}"]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        let ported = get_all_ported_comments(&test_repo.repo, change_id, sha).unwrap();
        let main_comments = &ported[Path::new("main.rs")];
        assert_eq!(main_comments.len(), 1);
        assert!(!main_comments[0].is_ported);
        assert_eq!(main_comments[0].ported_line, Some(2));
    }

    #[test]
    fn test_port_comments_shifted_lines() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("main.rs", "fn main() {\n    println!(\"hello\");\n}\n")
            .unwrap();
        let r1 = test_repo.commit("init").unwrap();
        let old_sha = r1.created.commit_id;
        let change_id = r1.created.change_id;

        // Comment on old version at line 2.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, old_sha).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 2,
                    start_line: None,
                    body: "nice print".to_string(),
                    anchor: make_anchor(&["fn main() {"], &["    println!(\"hello\");"], &["}"]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Rewrite the same change with lines added before the println.
        test_repo.edit(change_id).unwrap();
        test_repo
            .write_file(
                "main.rs",
                "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"hello\");\n}\n",
            )
            .unwrap();
        let new_info = test_repo.work_copy().unwrap();
        let new_sha = new_info.commit_id;
        assert_eq!(new_info.change_id, change_id);

        // Port to new SHA — the println moved from line 2 to line 4.
        let ported = get_all_ported_comments(&test_repo.repo, change_id, new_sha).unwrap();
        let main_comments = &ported[Path::new("main.rs")];
        assert_eq!(main_comments.len(), 1);
        assert!(main_comments[0].is_ported);
        assert_eq!(main_comments[0].ported_line, Some(4));
    }

    #[test]
    fn test_port_comments_deleted_file() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("temp.rs", "fn temp() {}\n").unwrap();
        let r1 = test_repo.commit("add temp").unwrap();
        let old_sha = r1.created.commit_id;
        let change_id = r1.created.change_id;

        // Comment on old version.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, old_sha).unwrap();
            cc.append_action(
                Path::new("temp.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 1,
                    start_line: None,
                    body: "remove this".to_string(),
                    anchor: make_anchor(&[], &["fn temp() {}"], &[]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Rewrite the same change, deleting the file.
        test_repo.edit(change_id).unwrap();
        test_repo.delete_file("temp.rs").unwrap();
        let new_info = test_repo.work_copy().unwrap();
        let new_sha = new_info.commit_id;
        assert_eq!(new_info.change_id, change_id);

        // Port — file gone, comment degrades to file-level.
        let ported = get_all_ported_comments(&test_repo.repo, change_id, new_sha).unwrap();
        let temp_comments = &ported[Path::new("temp.rs")];
        assert_eq!(temp_comments.len(), 1);
        assert!(temp_comments[0].is_ported);
        assert_eq!(temp_comments[0].ported_line, None);
    }

    #[test]
    fn test_port_comments_anchor_mismatch() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("main.rs", "fn main() {\n    println!(\"hello\");\n}\n")
            .unwrap();
        let r1 = test_repo.commit("init").unwrap();
        let old_sha = r1.created.commit_id;
        let change_id = r1.created.change_id;

        // Comment with anchor that won't match after rewrite.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, old_sha).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 2,
                    start_line: None,
                    body: "comment".to_string(),
                    anchor: make_anchor(&["fn main() {"], &["    println!(\"hello\");"], &["}"]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Rewrite the same change — completely different content, anchor won't match.
        test_repo.edit(change_id).unwrap();
        test_repo
            .write_file(
                "main.rs",
                "fn something_else() {\n    // totally different\n}\n",
            )
            .unwrap();
        let new_info = test_repo.work_copy().unwrap();
        let new_sha = new_info.commit_id;
        assert_eq!(new_info.change_id, change_id);

        let ported = get_all_ported_comments(&test_repo.repo, change_id, new_sha).unwrap();
        let main_comments = &ported[Path::new("main.rs")];
        assert_eq!(main_comments.len(), 1);
        assert!(main_comments[0].is_ported);
        // Anchor didn't match — degrades to file-level.
        assert_eq!(main_comments[0].ported_line, None);
    }

    #[test]
    fn test_port_multiline_comment() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file(
                "main.rs",
                "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n",
            )
            .unwrap();
        let r1 = test_repo.commit("init").unwrap();
        let old_sha = r1.created.commit_id;
        let change_id = r1.created.change_id;

        // Multi-line comment from line 2 to line 4 (start_line=2, line=4).
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, old_sha).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 4,
                    start_line: Some(2),
                    body: "this block".to_string(),
                    anchor: make_anchor(
                        &["fn main() {"],
                        &["    let a = 1;", "    let b = 2;", "    let c = 3;"],
                        &["}"],
                    ),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Rewrite the same change, adding a line before the block.
        test_repo.edit(change_id).unwrap();
        test_repo
            .write_file(
                "main.rs",
                "fn main() {\n    // comment\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n",
            )
            .unwrap();
        let new_info = test_repo.work_copy().unwrap();
        let new_sha = new_info.commit_id;
        assert_eq!(new_info.change_id, change_id);

        let ported = get_all_ported_comments(&test_repo.repo, change_id, new_sha).unwrap();
        let main_comments = &ported[Path::new("main.rs")];
        assert_eq!(main_comments.len(), 1);
        assert!(main_comments[0].is_ported);
        // Anchor target starts at line 3 now (shifted by 1).
        assert_eq!(main_comments[0].ported_line, Some(5)); // line 4 → 5
        assert_eq!(main_comments[0].ported_start_line, Some(3)); // start_line 2 → 3
    }

    #[test]
    fn test_port_from_multiple_old_shas() {
        let test_repo = TestRepo::new().unwrap();

        // v1: create the change.
        test_repo
            .write_file("main.rs", "line 1\nline 2\nline 3\n")
            .unwrap();
        let r1 = test_repo.commit("v1").unwrap();
        let sha_v1 = r1.created.commit_id;
        let change_id = r1.created.change_id;

        // Comment on v1 at line 2.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha_v1).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c1".to_string(),
                    side: DiffSide::New,
                    line: 2,
                    start_line: None,
                    body: "from v1".to_string(),
                    anchor: make_anchor(&["line 1"], &["line 2"], &["line 3"]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // v2: rewrite the same change, adding "line 4".
        test_repo.edit(change_id).unwrap();
        test_repo
            .write_file("main.rs", "line 1\nline 2\nline 3\nline 4\n")
            .unwrap();
        let v2_info = test_repo.work_copy().unwrap();
        let sha_v2 = v2_info.commit_id;
        assert_eq!(v2_info.change_id, change_id);

        // Comment on v2 at line 4.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha_v2).unwrap();
            cc.append_action(
                Path::new("main.rs"),
                CommentAction::Create {
                    comment_id: "c2".to_string(),
                    side: DiffSide::New,
                    line: 4,
                    start_line: None,
                    body: "from v2".to_string(),
                    anchor: make_anchor(&["line 3"], &["line 4"], &[]),
                },
            )
            .unwrap();
            cc.write().unwrap();
        }

        // v3: rewrite again, adding "line 0" at the start and "line 5" at the end.
        test_repo
            .write_file(
                "main.rs",
                "line 0\nline 1\nline 2\nline 3\nline 4\nline 5\n",
            )
            .unwrap();
        let v3_info = test_repo.work_copy().unwrap();
        let sha_v3 = v3_info.commit_id;
        assert_eq!(v3_info.change_id, change_id);

        // Port both old comments to v3.
        let ported = get_all_ported_comments(&test_repo.repo, change_id, sha_v3).unwrap();
        let main_comments = &ported[Path::new("main.rs")];
        assert_eq!(main_comments.len(), 2);

        let c1 = main_comments.iter().find(|c| c.comment.id == "c1").unwrap();
        let c2 = main_comments.iter().find(|c| c.comment.id == "c2").unwrap();

        assert!(c1.is_ported);
        assert_eq!(c1.ported_line, Some(3)); // "line 2" shifted from line 2 to line 3
        assert!(c2.is_ported);
        assert_eq!(c2.ported_line, Some(5)); // "line 4" shifted from line 4 to line 5
    }
}
