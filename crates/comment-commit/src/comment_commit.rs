use std::collections::HashMap;
use std::path::{Path, PathBuf};

use git2::{Commit, Repository, Signature, Tree};

use crate::comment_commit_lock::CommentCommitLock;
use crate::materialize::materialize;
use crate::model::{ActionEntry, AnchorContext, CommentAction, DiffSide, MaterializedComment};
use crate::tree_builder_ext::TreeBuilderExt;
use crate::{ChangeId, CommitId, Error, Result};

const ANCHOR_CONTEXT_LINES: usize = 3;

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

/// Manages inline diff comments for a specific (change_id, commit_sha) pair.
///
/// Comments are stored as an append-only action log in git objects:
/// - Ref: `refs/kenjutu/{change_id}/comments/{commit_sha}`
/// - Tree: each file path maps to a blob containing a JSON array of `ActionEntry`
/// - Commit parent: the code commit being commented on (prevents GC)
///
/// A file lock is held for the lifetime of this struct to prevent concurrent writes.
pub struct CommentCommit<'a> {
    change_id: ChangeId,
    target: Commit<'a>,
    actions: HashMap<PathBuf, Vec<ActionEntry>>,
    repo: &'a Repository,
    _guard: CommentCommitLock,
}

impl<'a> CommentCommit<'a> {
    /// Open or create a comment-commit for the given (change_id, sha) pair.
    ///
    /// If a ref already exists, loads the existing action log from the tree.
    /// If not, starts with an empty action map.
    ///
    /// Acquires an exclusive file lock for the duration.
    pub fn get(repo: &'a Repository, change_id: ChangeId, sha: CommitId) -> Result<Self> {
        let guard = CommentCommitLock::new(repo, change_id, sha)?;
        log::info!(
            "acquired lock for comment-commit: change_id={}, sha={}",
            change_id,
            sha
        );

        let target = repo.find_commit(sha.oid())?;
        let ref_name = comment_ref_name(change_id, sha);

        let actions = match repo.find_reference(&ref_name) {
            Ok(reference) => {
                let commit = reference.peel_to_commit()?;
                let tree = commit.tree()?;
                load_actions_from_tree(repo, &tree)?
            }
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    return Err(Error::Git(err));
                }
                HashMap::new()
            }
        };

        Ok(Self {
            change_id,
            target,
            actions,
            repo,
            _guard: guard,
        })
    }

    /// Get the raw action log for a specific file.
    pub(crate) fn get_file_actions(&self, file_path: &Path) -> Vec<ActionEntry> {
        self.actions.get(file_path).cloned().unwrap_or_default()
    }

    /// Get the materialized comments for a specific file (replays the action log).
    pub fn get_file_comments(&self, file_path: &Path) -> Vec<MaterializedComment> {
        let actions = self.get_file_actions(file_path);
        materialize(&actions)
    }

    /// Get all materialized comments across all files.
    pub fn get_all_comments(&self) -> HashMap<PathBuf, Vec<MaterializedComment>> {
        self.actions
            .iter()
            .map(|(path, actions)| (path.clone(), materialize(actions)))
            .collect()
    }

    /// Create a new top-level inline comment on a diff.
    ///
    /// Generates the anchor context automatically from the git tree and
    /// assigns a new UUID v4 as the comment ID.
    pub fn create_comment(
        &mut self,
        file_path: &Path,
        side: DiffSide,
        line: u32,
        start_line: Option<u32>,
        body: String,
    ) -> Result<()> {
        let anchor = self.build_anchor(file_path, side, line, start_line)?;
        self.append_action(
            file_path,
            CommentAction::Create {
                comment_id: uuid::Uuid::new_v4().to_string(),
                side,
                line,
                start_line,
                body,
                anchor,
            },
        )
    }

    /// Reply to an existing top-level comment (flat threads only).
    ///
    /// Assigns a new UUID v4 as the reply ID.
    pub fn reply_to_comment(
        &mut self,
        file_path: &Path,
        parent_comment_id: String,
        body: String,
    ) -> Result<()> {
        self.append_action(
            file_path,
            CommentAction::Reply {
                comment_id: uuid::Uuid::new_v4().to_string(),
                parent_comment_id,
                body,
            },
        )
    }

    /// Edit the body of an existing comment or reply.
    pub fn edit_comment(
        &mut self,
        file_path: &Path,
        comment_id: String,
        body: String,
    ) -> Result<()> {
        self.append_action(file_path, CommentAction::Edit { comment_id, body })
    }

    /// Resolve a comment thread (targets the root comment only).
    pub fn resolve_comment(&mut self, file_path: &Path, comment_id: String) -> Result<()> {
        self.append_action(file_path, CommentAction::Resolve { comment_id })
    }

    /// Unresolve a previously resolved comment thread (targets the root comment only).
    pub fn unresolve_comment(&mut self, file_path: &Path, comment_id: String) -> Result<()> {
        self.append_action(file_path, CommentAction::Unresolve { comment_id })
    }

    /// Build anchor context by reading file content from the git tree.
    ///
    /// For `DiffSide::New`, reads from the target commit's tree.
    /// For `DiffSide::Old`, reads from the target commit's parent tree.
    fn build_anchor(
        &self,
        file_path: &Path,
        side: DiffSide,
        line: u32,
        start_line: Option<u32>,
    ) -> Result<AnchorContext> {
        let tree = match side {
            DiffSide::New => self.target.tree()?,
            DiffSide::Old => {
                let parent = self.target.parent(0).map_err(|_| {
                    Error::Internal("cannot comment on old side of initial commit".into())
                })?;
                parent.tree()?
            }
        };

        let content = read_file_from_tree(self.repo, &tree, file_path).ok_or_else(|| {
            Error::Internal(format!("file not found in tree: {}", file_path.display()))
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();

        // Determine the target range (1-based → 0-based).
        let start_0 = start_line.unwrap_or(line).saturating_sub(1) as usize;
        let end_0 = line.saturating_sub(1) as usize;

        if start_0 >= total || end_0 >= total || start_0 > end_0 {
            return Err(Error::Internal(format!(
                "line range out of bounds: start={}, end={}, total={}",
                start_0 + 1,
                end_0 + 1,
                total
            )));
        }

        let before_start = start_0.saturating_sub(ANCHOR_CONTEXT_LINES);
        let after_end = (end_0 + 1 + ANCHOR_CONTEXT_LINES).min(total);

        Ok(AnchorContext {
            before: lines[before_start..start_0]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            target: lines[start_0..=end_0]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            after: lines[end_0 + 1..after_end]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    /// Append an action to the log for a specific file.
    ///
    /// Generates a new UUID v4 for the `action_id` and an ISO 8601 timestamp
    /// for `created_at`.
    ///
    /// Validates:
    /// - `Reply.parent_comment_id` must reference an existing `Create` action
    /// - `Resolve`/`Unresolve` must target a `Create` action (thread root)
    /// - `Edit` must target an existing `Create` or `Reply` action
    fn append_action(&mut self, file_path: &Path, action: CommentAction) -> Result<()> {
        // Validate before borrowing mutably.
        let existing = self.actions.get(file_path).map(|v| v.as_slice());
        validate_action(existing.unwrap_or(&[]), &action)?;

        let actions = self.actions.entry(file_path.to_path_buf()).or_default();
        let entry = ActionEntry {
            action_id: uuid::Uuid::new_v4().to_string(),
            created_at: now_iso8601(),
            action,
        };
        actions.push(entry);
        Ok(())
    }

    /// Write the current state to a git commit and update the ref.
    ///
    /// Returns the `CommitId` of the newly created comment-commit.
    pub fn write(&self) -> Result<CommitId> {
        let tree_oid = self.build_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let sha = CommitId::from(self.target.id());
        let message = format!(
            "update comments for change_id: {}, sha: {}",
            self.change_id, sha,
        );
        let signature = Self::signature()?;
        let oid = self.repo.commit(
            None,
            &signature,
            &signature,
            &message,
            &tree,
            &[&self.target],
        )?;
        log::info!(
            "created comment-commit {} for change_id={}, sha={}",
            oid,
            self.change_id,
            sha,
        );

        let ref_name = comment_ref_name(self.change_id, sha);
        let log_message = format!(
            "kenjutu: updated comment ref for change_id: {}, sha: {}",
            self.change_id, sha,
        );
        self.repo.reference(&ref_name, oid, true, &log_message)?;

        Ok(CommitId::from(oid))
    }

    fn build_tree(&self) -> Result<git2::Oid> {
        let ext = TreeBuilderExt::new(self.repo);

        // Start with an empty tree.
        let empty_oid = self.repo.treebuilder(None)?.write()?;
        let mut tree = self.repo.find_tree(empty_oid)?;

        for (file_path, actions) in &self.actions {
            if actions.is_empty() {
                continue;
            }
            let json = serde_json::to_vec_pretty(actions)?;
            let blob_oid = self.repo.blob(&json)?;
            let tree_oid =
                ext.insert_file(&tree, file_path, blob_oid, git2::FileMode::Blob.into())?;
            tree = self.repo.find_tree(tree_oid)?;
        }

        Ok(tree.id())
    }

    fn signature() -> Result<Signature<'static>> {
        let sig = Signature::now("kenjutu", "kenjutu@gmail.com")?;
        Ok(sig)
    }
}

/// Construct the ref name for a comment-commit.
pub fn comment_ref_name(change_id: ChangeId, sha: CommitId) -> String {
    format!("refs/kenjutu/{}/comments/{}", change_id, sha)
}

/// Enumerate all comment refs for a given change_id.
/// Returns a list of (commit_sha, ref_name) pairs.
pub fn enumerate_comment_refs(
    repo: &Repository,
    change_id: ChangeId,
) -> Result<Vec<(CommitId, String)>> {
    let prefix = format!("refs/kenjutu/{}/comments/", change_id);
    let mut results = Vec::new();

    for reference in repo.references_glob(&format!("{}*", prefix))? {
        let reference = reference?;
        if let Some(name) = reference.name() {
            let sha_str = name.strip_prefix(&prefix).unwrap_or("");
            if let Ok(sha) = sha_str.parse::<CommitId>() {
                results.push((sha, name.to_string()));
            }
        }
    }

    Ok(results)
}

/// Load action logs from a comment-commit tree.
/// Each tree entry at a file path maps to a blob containing JSON `Vec<ActionEntry>`.
fn load_actions_from_tree(
    repo: &Repository,
    tree: &Tree<'_>,
) -> Result<HashMap<PathBuf, Vec<ActionEntry>>> {
    let mut actions = HashMap::new();
    collect_tree_entries(repo, tree, &PathBuf::new(), &mut actions)?;
    Ok(actions)
}

/// Recursively walk a git tree, collecting blob entries as action logs.
fn collect_tree_entries(
    repo: &Repository,
    tree: &Tree<'_>,
    prefix: &Path,
    actions: &mut HashMap<PathBuf, Vec<ActionEntry>>,
) -> Result<()> {
    for entry in tree.iter() {
        let name = entry
            .name()
            .ok_or_else(|| Error::Internal("non-utf8 tree entry name".to_string()))?;
        let path = prefix.join(name);

        match entry.kind() {
            Some(git2::ObjectType::Blob) => {
                let blob = repo.find_blob(entry.id())?;
                let content = blob.content();
                let file_actions: Vec<ActionEntry> = serde_json::from_slice(content)?;
                actions.insert(path, file_actions);
            }
            Some(git2::ObjectType::Tree) => {
                let subtree = repo.find_tree(entry.id())?;
                collect_tree_entries(repo, &subtree, &path, actions)?;
            }
            _ => {
                // Skip unknown entry types.
            }
        }
    }
    Ok(())
}

/// Validate an action against the existing action log.
fn validate_action(existing_actions: &[ActionEntry], action: &CommentAction) -> Result<()> {
    match action {
        CommentAction::Create { .. } => {
            // No validation needed — duplicates are handled at materialization.
            Ok(())
        }
        CommentAction::Reply {
            parent_comment_id, ..
        } => {
            if !has_create_action(existing_actions, parent_comment_id) {
                return Err(Error::InvalidAction {
                    message: format!("Reply targets non-existent comment: {}", parent_comment_id,),
                });
            }
            Ok(())
        }
        CommentAction::Edit { comment_id, .. } => {
            if !has_create_action(existing_actions, comment_id)
                && !has_reply_action(existing_actions, comment_id)
            {
                return Err(Error::InvalidAction {
                    message: format!("Edit targets non-existent comment or reply: {}", comment_id,),
                });
            }
            Ok(())
        }
        CommentAction::Resolve { comment_id, .. } => {
            if !has_create_action(existing_actions, comment_id) {
                return Err(Error::InvalidAction {
                    message: format!("Resolve targets non-existent thread root: {}", comment_id,),
                });
            }
            Ok(())
        }
        CommentAction::Unresolve { comment_id, .. } => {
            if !has_create_action(existing_actions, comment_id) {
                return Err(Error::InvalidAction {
                    message: format!("Unresolve targets non-existent thread root: {}", comment_id,),
                });
            }
            Ok(())
        }
    }
}

/// Check if an action log contains a Create action with the given comment_id.
fn has_create_action(actions: &[ActionEntry], comment_id: &str) -> bool {
    actions.iter().any(|entry| {
        matches!(
            &entry.action,
            CommentAction::Create { comment_id: id, .. } if id == comment_id
        )
    })
}

/// Check if an action log contains a Reply action with the given comment_id.
fn has_reply_action(actions: &[ActionEntry], comment_id: &str) -> bool {
    actions.iter().any(|entry| {
        matches!(
            &entry.action,
            CommentAction::Reply { comment_id: id, .. } if id == comment_id
        )
    })
}

/// Generate the current UTC time as an ISO 8601 string.
fn now_iso8601() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = epoch_days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DiffSide;
    use test_repo::TestRepo;

    #[test]
    fn test_create_and_read_comment() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("src/main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("initial commit").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        // Create a comment.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("src/main.rs"),
                DiffSide::New,
                1,
                None,
                "looks good".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Read it back.
        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let comments = cc.get_file_comments(Path::new("src/main.rs"));
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].body, "looks good");
            assert_eq!(comments[0].line, 1);
        }
    }

    #[test]
    fn test_append_reply_and_read() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("lib.rs", "pub fn foo() {}").unwrap();
        let result = test_repo.commit("add lib").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("lib.rs"),
                DiffSide::New,
                1,
                None,
                "why public?".to_string(),
            )
            .unwrap();

            let comments = cc.get_file_comments(Path::new("lib.rs"));
            let comment_id = comments[0].id.clone();

            cc.reply_to_comment(Path::new("lib.rs"), comment_id, "for testing".to_string())
                .unwrap();
            cc.write().unwrap();
        }

        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let comments = cc.get_file_comments(Path::new("lib.rs"));
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].replies.len(), 1);
            assert_eq!(comments[0].replies[0].body, "for testing");
        }
    }

    #[test]
    fn test_edit_and_resolve() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("app.rs", "fn app() {}").unwrap();
        let result = test_repo.commit("add app").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        // Create + edit + resolve in one session.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("app.rs"),
                DiffSide::New,
                1,
                None,
                "original".to_string(),
            )
            .unwrap();

            let comments = cc.get_file_comments(Path::new("app.rs"));
            let comment_id = comments[0].id.clone();

            cc.edit_comment(
                Path::new("app.rs"),
                comment_id.clone(),
                "edited".to_string(),
            )
            .unwrap();
            cc.resolve_comment(Path::new("app.rs"), comment_id).unwrap();
            cc.write().unwrap();
        }

        // Read back and verify.
        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let comments = cc.get_file_comments(Path::new("app.rs"));
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].body, "edited");
            assert_eq!(comments[0].edit_count, 1);
            assert!(comments[0].resolved);
        }
    }

    #[test]
    fn test_multiple_files() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("a.rs", "fn a() {}").unwrap();
        test_repo.write_file("b.rs", "fn b() {}").unwrap();
        let result = test_repo.commit("add files").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("a.rs"),
                DiffSide::New,
                1,
                None,
                "comment on a".to_string(),
            )
            .unwrap();
            cc.create_comment(
                Path::new("b.rs"),
                DiffSide::New,
                1,
                None,
                "comment on b".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let a_comments = cc.get_file_comments(Path::new("a.rs"));
            let b_comments = cc.get_file_comments(Path::new("b.rs"));
            assert_eq!(a_comments.len(), 1);
            assert_eq!(a_comments[0].body, "comment on a");
            assert_eq!(b_comments.len(), 1);
            assert_eq!(b_comments[0].body, "comment on b");
        }
    }

    #[test]
    fn test_nested_file_path() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("src/services/auth.rs", "fn auth() {}")
            .unwrap();
        let result = test_repo.commit("add nested file").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("src/services/auth.rs"),
                DiffSide::New,
                1,
                None,
                "nested comment".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let comments = cc.get_file_comments(Path::new("src/services/auth.rs"));
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].body, "nested comment");
        }
    }

    #[test]
    fn test_append_across_sessions() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("main.rs", "line 1\nline 2\nline 3\nline 4\nline 5\n")
            .unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        // Session 1: create comment on line 1.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("main.rs"),
                DiffSide::New,
                1,
                None,
                "first comment".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Session 2: create comment on line 5.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("main.rs"),
                DiffSide::New,
                5,
                None,
                "second comment".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Session 3: read all.
        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let comments = cc.get_file_comments(Path::new("main.rs"));
            assert_eq!(comments.len(), 2);
            assert_eq!(comments[0].body, "first comment");
            assert_eq!(comments[1].body, "second comment");
        }
    }

    #[test]
    fn test_reply_to_nonexistent_parent_fails() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        let result = cc.reply_to_comment(
            Path::new("main.rs"),
            "nonexistent".to_string(),
            "orphan reply".to_string(),
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("non-existent comment")
        );
    }

    #[test]
    fn test_resolve_nonexistent_comment_fails() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        let result = cc.resolve_comment(Path::new("main.rs"), "nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_nonexistent_comment_fails() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        let result = cc.edit_comment(
            Path::new("main.rs"),
            "nonexistent".to_string(),
            "edited".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_enumerate_comment_refs() {
        let test_repo = TestRepo::new().unwrap();

        // Create a commit and record its SHA + change_id.
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let r1 = test_repo.commit("commit 1").unwrap();
        let change_id = r1.created.change_id;
        let old_sha = r1.created.commit_id;

        // Comment on the original SHA.
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, old_sha).unwrap();
            cc.create_comment(
                Path::new("main.rs"),
                DiffSide::New,
                1,
                None,
                "comment on v1".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        // Rewrite the same change (simulates a rebase), producing a new SHA
        // for the same change_id.
        test_repo.edit(change_id).unwrap();
        test_repo
            .write_file("main.rs", "fn main() { println!(\"hello\"); }")
            .unwrap();
        let new_info = test_repo.work_copy().unwrap();
        let new_sha = new_info.commit_id;
        assert_eq!(new_info.change_id, change_id);
        assert_ne!(new_sha, old_sha);

        // Comment on the rewritten SHA (same change_id, different commit SHA).
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, new_sha).unwrap();
            cc.create_comment(
                Path::new("main.rs"),
                DiffSide::New,
                1,
                None,
                "comment on v2".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        let refs = enumerate_comment_refs(&test_repo.repo, change_id).unwrap();
        assert_eq!(refs.len(), 2);

        let shas: Vec<CommitId> = refs.iter().map(|(sha, _)| *sha).collect();
        assert!(shas.contains(&old_sha));
        assert!(shas.contains(&new_sha));
    }

    #[test]
    fn test_comment_commit_parent_is_target() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let comment_sha;
        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("main.rs"),
                DiffSide::New,
                1,
                None,
                "test".to_string(),
            )
            .unwrap();
            comment_sha = cc.write().unwrap();
        }

        // Verify the comment-commit's parent is the target code commit.
        let comment_commit = test_repo.repo.find_commit(comment_sha.oid()).unwrap();
        assert_eq!(comment_commit.parent_count(), 1);
        assert_eq!(CommitId::from(comment_commit.parent_id(0).unwrap()), sha);
    }

    #[test]
    fn test_get_all_comments() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("a.rs", "fn a() {}").unwrap();
        test_repo.write_file("b.rs", "fn b() {}").unwrap();
        let result = test_repo.commit("add files").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        {
            let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            cc.create_comment(
                Path::new("a.rs"),
                DiffSide::New,
                1,
                None,
                "on a".to_string(),
            )
            .unwrap();
            cc.create_comment(
                Path::new("b.rs"),
                DiffSide::New,
                1,
                None,
                "on b".to_string(),
            )
            .unwrap();
            cc.write().unwrap();
        }

        {
            let cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
            let all = cc.get_all_comments();
            assert_eq!(all.len(), 2);
            assert!(all.contains_key(Path::new("a.rs")));
            assert!(all.contains_key(Path::new("b.rs")));
        }
    }

    #[test]
    fn test_build_anchor_generates_context() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file(
                "main.rs",
                "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\n",
            )
            .unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        cc.create_comment(
            Path::new("main.rs"),
            DiffSide::New,
            4,
            None,
            "middle line".to_string(),
        )
        .unwrap();

        let comments = cc.get_file_comments(Path::new("main.rs"));
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments[0].anchor.before,
            vec!["line 1", "line 2", "line 3"]
        );
        assert_eq!(comments[0].anchor.target, vec!["line 4"]);
        assert_eq!(comments[0].anchor.after, vec!["line 5", "line 6", "line 7"]);
    }

    #[test]
    fn test_build_anchor_multiline_target() {
        let test_repo = TestRepo::new().unwrap();
        test_repo
            .write_file("main.rs", "a\nb\nc\nd\ne\nf\ng\n")
            .unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        // Multi-line: start_line=3, line=5 → target is lines 3,4,5
        cc.create_comment(
            Path::new("main.rs"),
            DiffSide::New,
            5,
            Some(3),
            "block comment".to_string(),
        )
        .unwrap();

        let comments = cc.get_file_comments(Path::new("main.rs"));
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].anchor.before, vec!["a", "b"]);
        assert_eq!(comments[0].anchor.target, vec!["c", "d", "e"]);
        assert_eq!(comments[0].anchor.after, vec!["f", "g"]);
    }

    #[test]
    fn test_create_comment_old_side_initial_commit_fails() {
        let test_repo = TestRepo::new().unwrap();
        test_repo.write_file("main.rs", "fn main() {}").unwrap();
        let result = test_repo.commit("init").unwrap();
        let sha = result.created.commit_id;
        let change_id = result.created.change_id;

        let mut cc = CommentCommit::get(&test_repo.repo, change_id, sha).unwrap();
        let result = cc.create_comment(
            Path::new("main.rs"),
            DiffSide::Old,
            1,
            None,
            "old side".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("initial commit"));
    }
}
