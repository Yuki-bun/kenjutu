use std::collections::HashSet;
use std::path::PathBuf;

use git2::{Delta, Repository, Tree};
use kenjutu_types::{ChangeId, CommitId};
use marker_commit::MarkerCommit;

use super::{Error, Result};
use crate::models::{FileChangeStatus, FileEntry, ReviewStatus};
use crate::services::git;
use crate::services::jj::get_change_id;

fn map_delta_status(status: Delta) -> FileChangeStatus {
    match status {
        Delta::Added => FileChangeStatus::Added,
        Delta::Deleted => FileChangeStatus::Deleted,
        Delta::Modified => FileChangeStatus::Modified,
        Delta::Renamed => FileChangeStatus::Renamed,
        Delta::Copied => FileChangeStatus::Copied,
        Delta::Typechange => FileChangeStatus::Typechange,
        _ => FileChangeStatus::Modified,
    }
}

/// Extract metadata from a patch without fetching blob contents or syntax highlighting.
fn process_patch_metadata(patch: &git2::Patch, marker_tree: &Tree) -> Result<FileEntry> {
    let delta = patch.delta();
    let old_file = delta.old_file();
    let new_file = delta.new_file();

    // libgit2 sets new_file.path to the same path as old_file.path even for deletions,
    // so we use delta.status() (not new_file.path().is_none()) to detect deletions.
    let is_deletion = delta.status() == Delta::Deleted;

    let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
    let new_path = if is_deletion {
        None
    } else {
        new_file.path().map(|p| p.to_string_lossy().to_string())
    };

    let status = map_delta_status(delta.status());
    let is_binary = old_file.is_binary() || new_file.is_binary();

    let (_context, additions, deletions) = patch.line_stats()?;
    let (additions, deletions) = (additions as u32, deletions as u32);

    let review_status = if is_deletion {
        // Deletion: binary choice — M still has the file (Unreviewed) or doesn't (Reviewed).
        match marker_tree.get_path(old_file.path().unwrap()) {
            Ok(_) => ReviewStatus::Unreviewed,
            Err(err) if err.code() == git2::ErrorCode::NotFound => ReviewStatus::Reviewed,
            Err(e) => return Err(e.into()),
        }
    } else {
        // Addition, modification, rename, copy:
        // Compare M's blob at new_path against B's blob (old_file.id) and T's blob (new_file.id).
        // For additions, old_file.id() is the null OID — M can never hold a null-ID blob,
        // so the blob_b equality check is a no-op there and falls through to PartiallyReviewed.
        let target_path = new_file.path().unwrap();
        match marker_tree.get_path(target_path) {
            Ok(content) => {
                if content.id() == new_file.id() {
                    ReviewStatus::Reviewed
                } else if content.id() == old_file.id() {
                    ReviewStatus::Unreviewed
                } else {
                    ReviewStatus::PartiallyReviewed
                }
            }
            Err(err) if err.code() == git2::ErrorCode::NotFound => ReviewStatus::Unreviewed,
            Err(e) => return Err(e.into()),
        }
    };

    Ok(FileEntry {
        old_path,
        new_path,
        status,
        additions,
        deletions,
        is_binary,
        review_status,
    })
}

/// Generate a lightweight file list without blob fetching or syntax highlighting.
/// This is fast because it only iterates over diff deltas and counts lines from patches.
pub fn generate_file_list(
    repository: &git2::Repository,
    sha: CommitId,
) -> Result<(ChangeId, Vec<FileEntry>)> {
    let commit = repository
        .find_commit(sha.oid())
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;

    let change_id = git::get_change_id(&commit).map_or_else(
        || {
            get_change_id(repository.path().parent().unwrap(), &sha.to_string()).map_err(|e| {
                Error::Internal(format!("Failed to get change_id for non-jj commit: {e}"))
            })
        },
        Ok,
    )?;

    // Get commit tree and parent tree
    let commit_tree = commit.tree()?;
    let (base_tree, marker_tree) = {
        let marker_commit = MarkerCommit::get(repository, change_id, sha).map_err(|e| match e {
            marker_commit::Error::BasesMergeConflict { .. } => Error::MergeConflict(sha),
            e => Error::MarkerCommit(e),
        })?;
        (
            marker_commit.base_tree().clone(),
            marker_commit.marker_tree().clone(),
        )
    };

    let diff = diff_with_options(repository, &base_tree, &commit_tree)?;
    let base_to_marker_diff = diff_with_options(repository, &base_tree, &marker_tree)?;

    // Process all file deltas to extract metadata only.
    // Collect all paths touched by diff(B, T) so we can skip them in the ReviewedReverted pass.
    let mut files: Vec<FileEntry> = Vec::new();
    let mut bt_paths: HashSet<PathBuf> = HashSet::new();
    for (delta_idx, delta) in diff.deltas().enumerate() {
        if let Some(p) = delta.old_file().path() {
            bt_paths.insert(p.to_path_buf());
        }
        if let Some(p) = delta.new_file().path() {
            bt_paths.insert(p.to_path_buf());
        }
        let patch = git2::Patch::from_diff(&diff, delta_idx)?;
        if let Some(patch) = patch {
            files.push(process_patch_metadata(&patch, &marker_tree)?);
        }
    }

    // ReviewedReverted pass: files in diff(B, M) that are no longer in diff(B, T).
    // These were previously reviewed but reverted back to base content.
    for delta in base_to_marker_diff.deltas() {
        let is_deletion = delta.status() == Delta::Deleted;
        let old_path = delta.old_file().path().map(|p| p.to_path_buf());
        // libgit2 sets new_file.path to old_file.path for deletions; suppress it here.
        let new_path = if is_deletion {
            None
        } else {
            delta.new_file().path().map(|p| p.to_path_buf())
        };
        let already_in_bt = old_path.as_deref().is_some_and(|p| bt_paths.contains(p))
            || new_path.as_deref().is_some_and(|p| bt_paths.contains(p));
        if already_in_bt {
            continue;
        }
        files.push(FileEntry {
            old_path: old_path.map(|p| p.to_string_lossy().into_owned()),
            new_path: new_path.map(|p| p.to_string_lossy().into_owned()),
            status: map_delta_status(delta.status()),
            additions: 0,
            deletions: 0,
            is_binary: delta.old_file().is_binary() || delta.new_file().is_binary(),
            review_status: ReviewStatus::ReviewedReverted,
        });
    }

    Ok((change_id, files))
}

fn diff_with_options<'repo>(
    repo: &'repo Repository,
    old_tree: &Tree<'repo>,
    new_tree: &Tree<'repo>,
) -> Result<git2::Diff<'repo>> {
    let mut opts = git2::DiffOptions::new();
    opts.context_lines(3)
        .interhunk_lines(0)
        .ignore_whitespace(false);

    let mut diff = repo.diff_tree_to_tree(Some(old_tree), Some(new_tree), Some(&mut opts))?;
    let mut find_opts = git2::DiffFindOptions::new();
    find_opts.renames(true);
    diff.find_similar(Some(&mut find_opts))?;
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::models::FileChangeStatus;
    use test_repo::TestRepo;

    #[test]
    fn file_list_added_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("hello.rs", "fn main() {}\n").unwrap();
        let commit = t.commit("add hello.rs").unwrap().created;

        let (change_id, files) = generate_file_list(&t.repo, commit.commit_id).unwrap();

        assert_eq!(change_id, commit.change_id);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Added);
        assert_eq!(files[0].new_path.as_deref(), Some("hello.rs"));
        assert!(files[0].additions > 0);
        assert_eq!(files[0].deletions, 0);
        assert!(!files[0].is_binary);
        assert_eq!(files[0].review_status, ReviewStatus::Unreviewed);
    }

    #[test]
    fn file_list_modified_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("lib.rs", "fn old() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.write_file("lib.rs", "fn new() {}\nfn extra() {}\n")
            .unwrap();
        let sha = t.commit("modify").unwrap().created.commit_id;

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Modified);
        assert_eq!(files[0].old_path.as_deref(), Some("lib.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("lib.rs"));
        assert!(files[0].additions > 0);
        assert!(files[0].deletions > 0);
    }

    #[test]
    fn file_list_deleted_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("temp.rs", "fn gone() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.delete_file("temp.rs").unwrap();
        let sha = t.commit("delete").unwrap().created.commit_id;

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Deleted);
        assert_eq!(files[0].old_path.as_deref(), Some("temp.rs"));
        assert_eq!(files[0].additions, 0);
        assert!(files[0].deletions > 0);
    }

    #[test]
    fn file_list_renamed_file() {
        // Use 10+ lines so git2 rename detection has enough content to match
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n\
                        line 6\nline 7\nline 8\nline 9\nline 10\n\
                        line 11\nline 12\n";
        let t = TestRepo::new().unwrap();

        t.write_file("old_name.rs", content).unwrap();
        t.commit("initial").unwrap();
        t.rename_file("old_name.rs", "new_name.rs").unwrap();
        let sha = t.commit("rename").unwrap().created.commit_id;

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Renamed);
        assert_eq!(files[0].old_path.as_deref(), Some("old_name.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new_name.rs"));
    }

    #[test]
    fn file_list_multiple_files() {
        let t = TestRepo::new().unwrap();
        t.write_file("a.rs", "a\n").unwrap();
        t.write_file("b.rs", "b\n").unwrap();
        t.write_file("c.rs", "c\n").unwrap();
        t.commit("initial").unwrap();

        t.write_file("a.rs", "aa\n").unwrap();
        t.write_file("b.rs", "bb\n").unwrap();
        t.write_file("c.rs", "cc\n").unwrap();
        let sha = t.commit("modify all").unwrap().created.commit_id;

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files.len(), 3);
        let mut paths: Vec<_> = files.iter().filter_map(|f| f.new_path.as_deref()).collect();
        paths.sort();
        assert_eq!(paths, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn file_list_addition_deletion_counts() {
        let t = TestRepo::new().unwrap();
        t.write_file("count.txt", "line1\nline2\nline3\nline4\nline5\n")
            .unwrap();
        t.commit("initial").unwrap();

        // Change 2 lines (line1, line2) and add 1 new line → 3 additions, 2 deletions
        t.write_file("count.txt", "LINE1\nLINE2\nline3\nline4\nline5\nnew line\n")
            .unwrap();
        let sha = t.commit("modify").unwrap().created.commit_id;

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files[0].additions, 3);
        assert_eq!(files[0].deletions, 2);
    }

    #[test]
    fn can_work_with_non_jj_commit() {
        let t = TestRepo::new().unwrap();
        t.write_file("hello", "world").unwrap();
        t.git_commit("initial").unwrap();
        t.write_file("hello", "everyone").unwrap();
        let sha = t.git_commit("modify").unwrap();
        let change_id = get_change_id(t.repo.path().parent().unwrap(), &sha.to_string()).unwrap();

        let (change_id_, files) = generate_file_list(&t.repo, sha).unwrap();
        assert_eq!(change_id_, change_id);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Modified);
        assert_eq!(files[0].old_path.as_deref(), Some("hello"));
        assert_eq!(files[0].new_path.as_deref(), Some("hello"));
        assert_eq!(files[0].review_status, ReviewStatus::Unreviewed);
    }

    // ── merge commit tests ──────────────────────────────────────────────

    #[test]
    fn pure_merge_has_empty_file_list() {
        let t = TestRepo::new().unwrap();
        // Commit A: file_a.txt
        t.write_file("file_a.txt", "hello\n").unwrap();
        let a = t.commit("add file_a").unwrap().created;
        // Commit B (child of A): adds file_b.txt
        t.write_file("file_b.txt", "world\n").unwrap();
        let b = t.commit("add file_b").unwrap().created;

        let merge_sha = t
            .merge(&[a.change_id, b.change_id], "merge")
            .unwrap()
            .commit_id;

        let (_, files) = generate_file_list(&t.repo, merge_sha).unwrap();

        assert!(
            files.is_empty(),
            "pure merge should have empty file list, got {} files: {:?}",
            files.len(),
            files.iter().map(|f| &f.new_path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn merge_with_conflict_resolution_shows_resolved_file() {
        //   M
        //  / \
        // B   C
        // \  /
        //  A
        let t = TestRepo::new().unwrap();
        t.write_file("file.txt", "base\n").unwrap();
        let a = t.commit("base").unwrap().created;
        t.write_file("file.txt", "from-branch\n").unwrap();
        let b = t.commit("branch").unwrap().created;

        let c = t.new_revision(a.change_id).unwrap();
        t.write_file("file.txt", "from-main\n").unwrap();

        // Merge M: parents=[B, C], tree has manually resolved content
        t.merge(&[b.change_id, c.change_id], "merge").unwrap();
        t.write_file("file.txt", "resolved\n").unwrap();
        let merge = t.work_copy().unwrap();

        let (_, files) = generate_file_list(&t.repo, merge.commit_id).unwrap();

        assert_eq!(
            files.len(),
            1,
            "merge with conflict resolution should show 1 file"
        );
    }

    #[test]
    fn merge_both_parents_modify_same_file_no_conflict() {
        // Both parents modify the same file in non-conflicting regions.
        // The auto-merged result differs from ALL parents, but it's still
        // a pure merge — no manual intervention needed.
        let original: String = (1..=20).map(|i| format!("line {i}\n")).collect();

        let mut branch_lines: Vec<String> = (1..=20).map(|i| format!("line {i}\n")).collect();
        branch_lines[2] = "CHANGED-BY-BRANCH line 3\n".to_string();
        let branch_content: String = branch_lines.concat();

        let mut main_lines: Vec<String> = (1..=20).map(|i| format!("line {i}\n")).collect();
        main_lines[17] = "CHANGED-BY-MAIN line 18\n".to_string();
        let main_content: String = main_lines.concat();

        let t = TestRepo::new().unwrap();
        // Ancestor commit A
        t.write_file("file.txt", &original).unwrap();
        let a = t.commit("ancestor").unwrap().created;
        // Branch commit B (child of A)
        t.write_file("file.txt", &branch_content).unwrap();
        let b = t.commit("branch change").unwrap().created;
        // Main commit C (child of A, via commit_merge with single parent)
        t.new_revision(a.change_id).unwrap();
        t.write_file("file.txt", &main_content).unwrap();
        let c = t.commit("main change").unwrap().created;

        // Merge M: parents=[B, C], tree = auto-merged (both changes)
        let merge = t.merge(&[b.change_id, c.change_id], "merge").unwrap();

        let (_, files) = generate_file_list(&t.repo, merge.commit_id).unwrap();

        assert!(
            files.is_empty(),
            "auto-merge with no conflicts should have empty file list, got {} files",
            files.len(),
        );
    }

    // ── review_status tests ────────────────────────────────────────────

    #[test]
    fn review_status_reviewed_after_marking_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("foo.rs", "fn old() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.write_file("foo.rs", "fn new() {}\n").unwrap();
        let b = t.commit("modify").unwrap().created;

        let mut marker =
            marker_commit::MarkerCommit::get(&t.repo, b.change_id, b.commit_id).unwrap();
        marker
            .mark_file_reviewed(Path::new("foo.rs"), None)
            .unwrap();
        marker.write().unwrap();
        drop(marker);

        let (_, files) = generate_file_list(&t.repo, b.commit_id).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].review_status, ReviewStatus::Reviewed);
    }

    #[test]
    fn review_status_partially_reviewed_after_one_hunk() {
        // Base: a1..a5, b1..b5 (10 lines); target: A1..a5, b1..B4..b5 (two hunks changed)
        let base_content = "a1\na2\na3\na4\na5\nb1\nb2\nb3\nb4\nb5\n";
        let target_content = "A1\na2\na3\na4\na5\nb1\nb2\nb3\nB4\nb5\n";

        let t = TestRepo::new().unwrap();
        t.write_file("test.rs", base_content).unwrap();
        t.commit("initial").unwrap();
        t.write_file("test.rs", target_content).unwrap();
        let b = t.commit("two hunks").unwrap().created;

        // Mark only hunk1 (@@ -1,3 +1,3 @@) in M/T space (M == B initially)
        let hunk1 = marker_commit::HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        let mut marker =
            marker_commit::MarkerCommit::get(&t.repo, b.change_id, b.commit_id).unwrap();
        marker
            .mark_hunk_reviewed(Path::new("test.rs"), None, &hunk1)
            .unwrap();
        marker.write().unwrap();
        drop(marker);

        let (_, files) = generate_file_list(&t.repo, b.commit_id).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].review_status, ReviewStatus::PartiallyReviewed);
    }

    #[test]
    fn review_status_deletion_reviewed() {
        let t = TestRepo::new().unwrap();
        t.write_file("gone.rs", "fn old() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.delete_file("gone.rs").unwrap();
        let b = t.commit("delete").unwrap().created;

        let mut marker =
            marker_commit::MarkerCommit::get(&t.repo, b.change_id, b.commit_id).unwrap();
        marker
            .mark_file_reviewed(Path::new("gone.rs"), None)
            .unwrap();
        marker.write().unwrap();
        drop(marker);

        let (_, files) = generate_file_list(&t.repo, b.commit_id).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].review_status, ReviewStatus::Reviewed);
    }

    #[test]
    fn review_status_reviewed_reverted() {
        // Commit B modifies "foo.rs"; we review it; then amend B to revert "foo.rs" back to base.
        // generate_file_list should emit a ReviewedReverted entry with 0 additions/deletions.
        let t = TestRepo::new().unwrap();
        t.write_file("foo.rs", "fn old() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.write_file("foo.rs", "fn new() {}\n").unwrap();
        let b = t.commit("modify").unwrap().created;

        // Mark reviewed
        let mut marker =
            marker_commit::MarkerCommit::get(&t.repo, b.change_id, b.commit_id).unwrap();
        marker
            .mark_file_reviewed(Path::new("foo.rs"), None)
            .unwrap();
        marker.write().unwrap();
        drop(marker);

        // Amend B: revert foo.rs back to base content so diff(B, T) becomes empty for this file
        t.edit(b.change_id).unwrap();
        t.write_file("foo.rs", "fn old() {}\n").unwrap();
        let b2 = t.work_copy().unwrap();

        let (_, files) = generate_file_list(&t.repo, b2.commit_id).unwrap();

        // diff(B, T) is now empty (no changes), but diff(B, M) still has foo.rs
        let reverted: Vec<_> = files
            .iter()
            .filter(|f| f.review_status == ReviewStatus::ReviewedReverted)
            .collect();
        assert_eq!(reverted.len(), 1, "expected one ReviewedReverted entry");
        assert_eq!(reverted[0].additions, 0);
        assert_eq!(reverted[0].deletions, 0);
        // No normal diff entries
        assert!(
            files
                .iter()
                .all(|f| f.review_status == ReviewStatus::ReviewedReverted),
            "all entries should be ReviewedReverted when the only change was reverted"
        );
    }
}
