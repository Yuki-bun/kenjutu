use git2::Delta;
use marker_commit::MarkerCommit;
use std::collections::HashSet;
use std::path::PathBuf;

use super::Result;
use crate::models::{ChangeId, FileChangeStatus, FileEntry};
use crate::services::git;

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
fn process_patch_metadata(
    patch: &git2::Patch,
    un_reviewed_files: &HashSet<PathBuf>,
) -> Result<FileEntry> {
    let delta = patch.delta();
    let old_file = delta.old_file();
    let new_file = delta.new_file();

    let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
    let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

    let status = map_delta_status(delta.status());
    let is_binary = old_file.is_binary() || new_file.is_binary();

    let (_context, additions, deletions) = patch.line_stats()?;
    let (additions, deletions) = (additions as u32, deletions as u32);

    let file_path = new_path.as_ref().or(old_path.as_ref()).map(PathBuf::from);
    let is_reviewed = file_path
        .as_ref()
        .map(|p| !un_reviewed_files.contains(p))
        .unwrap_or(false);
    Ok(FileEntry {
        old_path,
        new_path,
        status,
        additions,
        deletions,
        is_binary,
        is_reviewed,
    })
}

/// Generate a lightweight file list without blob fetching or syntax highlighting.
/// This is fast because it only iterates over diff deltas and counts lines from patches.
pub fn generate_file_list(
    repository: &git2::Repository,
    sha: git2::Oid,
) -> Result<(Option<ChangeId>, Vec<FileEntry>)> {
    let commit = repository
        .find_commit(sha)
        .map_err(|_| git::Error::CommitNotFound(sha.to_string()))?;

    // Extract change_id from commit
    let change_id = git::get_change_id(&commit);

    // Get commit tree and parent tree
    let commit_tree = commit.tree()?;

    // For merge commits, use auto-merged tree as base; otherwise use parent(0)
    let parent_tree = if commit.parent_count() > 0 {
        super::compute_merge_base_tree(repository, &commit)?
            .or_else(|| commit.parent(0).ok().and_then(|p| p.tree().ok()))
    } else {
        None
    };

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts
        .context_lines(3)
        .interhunk_lines(0)
        .ignore_whitespace(false);

    // Enable rename detection
    let mut find_opts = git2::DiffFindOptions::new();
    find_opts.renames(true);

    let mut diff = repository.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        Some(&mut diff_opts),
    )?;

    // Apply rename detection
    diff.find_similar(Some(&mut find_opts))?;

    let un_reviewed_files = if let Some(change_id) = &change_id {
        let marker_commit = MarkerCommit::get(
            repository,
            &marker_commit::ChangeId::from(change_id.as_str().to_string()),
            sha,
        )?;
        marker_commit.write()?;
        marker_commit.un_reviewed_files()?
    } else {
        // This is wrong but we will assign change_id to all commits in the future
        // so we will leave it like this for now.
        HashSet::new()
    };

    // Process all file deltas to extract metadata only
    let mut files: Vec<FileEntry> = Vec::new();
    for (delta_idx, _) in diff.deltas().enumerate() {
        let patch = git2::Patch::from_diff(&diff, delta_idx)?;
        if let Some(patch) = patch {
            files.push(process_patch_metadata(&patch, &un_reviewed_files)?);
        }
    }

    Ok((change_id, files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileChangeStatus;
    use test_repo::TestRepo;

    #[test]
    fn file_list_added_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("hello.rs", "fn main() {}\n").unwrap();
        let commit = t.commit("add hello.rs").unwrap().created;
        let sha = commit.oid();

        let (change_id, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(change_id.unwrap().as_str(), commit.change_id);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileChangeStatus::Added);
        assert_eq!(files[0].new_path.as_deref(), Some("hello.rs"));
        assert!(files[0].additions > 0);
        assert_eq!(files[0].deletions, 0);
        assert!(!files[0].is_binary);
        assert!(!files[0].is_reviewed);
    }

    #[test]
    fn file_list_modified_file() {
        let t = TestRepo::new().unwrap();
        t.write_file("lib.rs", "fn old() {}\n").unwrap();
        t.commit("initial").unwrap();
        t.write_file("lib.rs", "fn new() {}\nfn extra() {}\n")
            .unwrap();
        let sha = t.commit("modify").unwrap().created.oid();

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
        let sha = t.commit("delete").unwrap().created.oid();

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
        let sha = t.commit("rename").unwrap().created.oid();

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
        let sha = t.commit("modify all").unwrap().created.oid();

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
        let sha = t.commit("modify").unwrap().created.oid();

        let (_, files) = generate_file_list(&t.repo, sha).unwrap();

        assert_eq!(files[0].additions, 3);
        assert_eq!(files[0].deletions, 2);
    }

    // ── merge commit tests ──────────────────────────────────────────────

    #[test]
    fn pure_merge_has_empty_file_list() {
        let t = TestRepo::new().unwrap();
        // Commit A: file_a.txt
        t.write_file("file_a.txt", "hello\n").unwrap();
        let sha_a = t.commit("add file_a").unwrap().created.commit_id;
        // Commit B (child of A): adds file_b.txt
        t.write_file("file_b.txt", "world\n").unwrap();
        let sha_b = t.commit("add file_b").unwrap().created.commit_id;

        let merge_sha = t.merge(&[&sha_a, &sha_b], "merge").unwrap().oid();

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
        let sha_a = t.commit("base").unwrap().created.commit_id;
        t.write_file("file.txt", "from-branch\n").unwrap();
        let sha_b = t.commit("branch").unwrap().created.commit_id;

        let sha_c = t.new_revision(&sha_a).unwrap().commit_id;
        t.write_file("file.txt", "from-main\n").unwrap();

        // Merge M: parents=[C, B], tree has manually resolved content
        t.merge(&[&sha_b, &sha_c], "merge").unwrap();
        t.write_file("file.txt", "resolved\n").unwrap();
        let merge_sha = t.work_copy().unwrap().oid();

        let (_, files) = generate_file_list(&t.repo, merge_sha).unwrap();

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
        let sha_a = t.commit("ancestor").unwrap().created.commit_id;
        // Branch commit B (child of A)
        t.write_file("file.txt", &branch_content).unwrap();
        let sha_b = t.commit("branch change").unwrap().created.commit_id;
        // Main commit C (child of A, via commit_merge with single parent)
        t.new_revision(&sha_a).unwrap();
        t.write_file("file.txt", &main_content).unwrap();
        let sha_c = t.commit("main change").unwrap().created.commit_id;

        // Merge M: parents=[C, B], tree = auto-merged (both changes)
        let merge_sha = t.merge(&[&sha_b, &sha_c], "merge").unwrap().oid();

        let (_, files) = generate_file_list(&t.repo, merge_sha).unwrap();

        assert!(
            files.is_empty(),
            "auto-merge with no conflicts should have empty file list, got {} files",
            files.len(),
        );
    }
}
