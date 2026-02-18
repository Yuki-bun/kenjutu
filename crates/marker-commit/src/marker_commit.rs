use crate::{
    ChangeId, Error, Result, conflict::resolve_conflict_prefer_our,
    marker_commit_lock::MarkerCommitLock, tree_builder_ext::TreeBuilderExt,
};
use git2::{Commit, Oid, Repository, Signature, Tree};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// Commit for tracking review state for a specific revision.
/// Stored at refs/kenjutu/{change_id}/marker pointing to the parent of the revision being reviewed.
pub struct MarkerCommit<'a> {
    change_id: ChangeId,
    tree: Tree<'a>,
    target_tree: Tree<'a>,
    base: Option<Commit<'a>>,
    repo: &'a Repository,
    _guard: MarkerCommitLock,
}

impl<'a> std::fmt::Debug for MarkerCommit<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkerCommit")
            .field("change_id", &self.change_id)
            .field("base", &self.base)
            .finish()
    }
}

impl<'a> MarkerCommit<'a> {
    pub fn get(repo: &'a Repository, change_id: &ChangeId, sha: Oid) -> Result<Self> {
        let lock_file = MarkerCommitLock::new(repo, change_id.clone())?;
        log::info!(
            "acquired lock for marker commit for revision: {}",
            change_id.as_ref()
        );
        let target_commit = repo.find_commit(sha)?;

        if target_commit.parent_count() == 0 {
            let tree =
                if let Ok(reference) = repo.find_reference(&marker_commit_ref_name(change_id)) {
                    let marker_commit = reference.peel_to_commit()?;
                    marker_commit.tree()?
                } else {
                    let oid = empty_tree(repo)?;
                    repo.find_tree(oid)?
                };
            return Ok(Self {
                _guard: lock_file,
                tree,
                target_tree: target_commit.tree()?,
                base: None,
                repo,
                change_id: change_id.clone(),
            });
        }

        // TODO: handle merge commit
        let base = target_commit.parent(0)?;

        let ref_name = marker_commit_ref_name(change_id);
        let marker_tree = match repo.find_reference(&ref_name) {
            Ok(reference) => {
                let marker_commit = reference.peel_to_commit()?;
                let old_marker_base = marker_commit.parent(0)?;
                if old_marker_base.id() == base.id() {
                    marker_commit.tree()?
                } else {
                    let old_base_tree = old_marker_base.tree()?;
                    let new_base_tree = base.tree()?;
                    let marker_tree = marker_commit.tree()?;
                    let mut index =
                        repo.merge_trees(&old_base_tree, &new_base_tree, &marker_tree, None)?;
                    if index.has_conflicts() {
                        log::info!(
                            "marker commit for revision {{ change_id: {}, old_base: {}  }} has conflicted while rebasing onto new base {}.
                            Now resolving by preferring new base in the conflicted regions",
                            change_id.as_ref(),
                            old_marker_base.id(),
                            base.id()
                        );
                        println!("conflict");
                        let resolved_tree_oid = resolve_conflict_prefer_our(repo, &mut index)?;
                        repo.find_tree(resolved_tree_oid)?
                    } else {
                        let marker_tree_oid = index.write_tree_to(repo)?;
                        repo.find_tree(marker_tree_oid)?
                    }
                }
            }
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    log::warn!("Encountered unexpected error: {err}. Continuing anyway");
                }
                base.tree()?
            }
        };

        Ok(Self {
            _guard: lock_file,
            tree: marker_tree,
            base: Some(base),
            target_tree: target_commit.tree()?,
            repo,
            change_id: change_id.clone(),
        })
    }

    /// Mark a file as reviewed.
    /// # Args
    /// * `file_path` - path of the file to be marked as reviewed.
    ///   If the file is deleted in the target commit, pass the old path. Otherwise, pass the new path.
    /// * `old_path` - if the file is renamed, the old path of the file.
    pub fn mark_file_reviewed(&mut self, file_path: &Path, old_path: Option<&Path>) -> Result<()> {
        let ext = TreeBuilderExt::new(self.repo);

        // rename: remove old file and add new file
        if let Some(old_path) = old_path {
            let new_file = self.target_tree.get_path(file_path)?;
            let tree_after_remove = ext.remove_path(&self.tree, old_path)?;
            let tree = self.repo.find_tree(tree_after_remove)?;
            let new_tree_oid =
                ext.insert_file(&tree, file_path, new_file.id(), new_file.filemode())?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
            return Ok(());
        }

        match self.target_tree.get_path(file_path) {
            // Modification or addition
            Ok(target_content) => {
                let new_tree_oid = ext.insert_file(
                    &self.tree,
                    file_path,
                    target_content.id(),
                    target_content.filemode(),
                )?;
                self.tree = self.repo.find_tree(new_tree_oid)?;
            }
            // Deletion
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    return Err(Error::Git(err));
                }
                let new_tree_oid = ext.remove_path(&self.tree, file_path)?;
                self.tree = self.repo.find_tree(new_tree_oid)?;
            }
        }

        Ok(())
    }

    /// Mark a file as reviewed.
    /// # Args
    /// * `file_path` - path of the file to be marked as reviewed.
    ///   If the file is deleted in the target commit, pass the old path. Otherwise, pass the new path.
    /// * `old_path` - if the file is renamed, the old path of the file.
    pub fn unmark_file_reviewed(
        &mut self,
        file_path: &Path,
        old_path: Option<&Path>,
    ) -> Result<()> {
        let ext = TreeBuilderExt::new(self.repo);

        // rename: revert old file from base and remove new file from tree
        if let Some(old_path) = old_path {
            let Some(base) = &self.base else {
                log::warn!(
                    "encountered rename for initial commit. This should not happen. Continuing anyway"
                );
                // Should we error here?
                return Ok(());
            };
            let old_content = base.tree()?.get_path(old_path)?;
            let tree_after_insert = ext.insert_file(
                &self.tree,
                old_path,
                old_content.id(),
                old_content.filemode(),
            )?;
            let tree = self.repo.find_tree(tree_after_insert)?;
            let new_tree_oid = ext.remove_path(&tree, file_path)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
            return Ok(());
        }
        let Some(base) = &self.base else {
            // Initial commit means all files are added. So un-marking just means removing the file
            // from the tree.
            let new_tree_oid = ext.remove_path(&self.tree, file_path)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
            return Ok(());
        };

        match base.tree()?.get_path(file_path) {
            // Revert modified file
            Ok(target_content) => {
                let new_tree_oid = ext.insert_file(
                    &self.tree,
                    file_path,
                    target_content.id(),
                    target_content.filemode(),
                )?;
                self.tree = self.repo.find_tree(new_tree_oid)?;
            }
            // Revert added file
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    return Err(Error::Git(err));
                }
                let new_tree_oid = ext.remove_path(&self.tree, file_path)?;
                self.tree = self.repo.find_tree(new_tree_oid)?;
            }
        }

        Ok(())
    }

    /// Returns a map of file path to review status for all files in the target commit. `true`
    /// means reviewed, `false` means not reviewed.
    pub fn un_reviewed_files(&self) -> Result<HashSet<PathBuf>> {
        let mut diff =
            self.repo
                .diff_tree_to_tree(Some(&self.tree), Some(&self.target_tree), None)?;

        let mut diff_opts = git2::DiffFindOptions::new();
        diff_opts.renames(true);

        diff.find_similar(Some(&mut diff_opts))?;

        let mut un_reviewed_files = HashSet::new();
        for delta in diff.deltas() {
            let new_file = delta.new_file();
            let old_file = delta.old_file();
            let path = match delta.status() {
                git2::Delta::Added
                | git2::Delta::Renamed
                | git2::Delta::Modified
                | git2::Delta::Copied
                | git2::Delta::Conflicted => new_file.path().unwrap().to_path_buf(),
                git2::Delta::Deleted | git2::Delta::Ignored => {
                    old_file.path().unwrap().to_path_buf()
                }
                git2::Delta::Typechange
                | git2::Delta::Unreadable
                | git2::Delta::Untracked
                | git2::Delta::Unmodified => continue,
            };
            un_reviewed_files.insert(path);
        }
        Ok(un_reviewed_files)
    }

    /// Write the review status to the repository. Should be called after marking files as
    /// reviewed.
    /// Return the`Oid` of the marker commit.
    pub fn write(&self) -> Result<Oid> {
        let message = format!(
            "update marker commit for change_id: {}",
            self.change_id.as_ref()
        );
        let signature = Self::signature()?;
        let oid = if let Some(base) = &self.base {
            self.repo
                .commit(None, &signature, &signature, &message, &self.tree, &[base])?
        } else {
            self.repo
                .commit(None, &signature, &signature, &message, &self.tree, &[])?
        };
        log::info!("created marker commit for {}", self.change_id.as_ref());

        let ref_name = marker_commit_ref_name(&self.change_id);
        log::info!("Updating ref: {}", &ref_name);
        let log_message = format!(
            "kenjutu: updated reference for marker commit for change_id: {}",
            self.change_id.as_ref()
        );
        let force_update = true;
        self.repo
            .reference(&ref_name, oid, force_update, &log_message)?;
        Ok(oid)
    }

    fn signature() -> Result<Signature<'static>> {
        let sig = Signature::now("kenjutu", "kenjutu@gmail.com")?;
        Ok(sig)
    }
}

fn empty_tree(repo: &Repository) -> Result<Oid> {
    let builder = repo.treebuilder(None)?;
    let oid = builder.write()?;
    Ok(oid)
}

fn marker_commit_ref_name(change_id: &ChangeId) -> String {
    format!("refs/kenjutu/{}/marker", change_id.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;
    use test_repo::{CommitInfo, TestRepo};

    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// B add "test2" "hello world"
    /// |
    /// A add "test" "hello"
    fn setup_two_commits() -> Result<(TestRepo, CommitInfo, CommitInfo)> {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        let a = repo.commit("commit A")?.created;
        repo.write_file("test2", "hello world")?;
        let b = repo.commit("commit B")?.created;
        Ok((repo, a, b))
    }

    fn change_id(s: &str) -> ChangeId {
        ChangeId::from(s.to_string())
    }

    // ── MarkerCommit::get tests ────────────────────────────────────────

    #[test]
    fn create_marker_commit() -> Result {
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id("test");
        let marker_commit = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        marker_commit.write()?;

        assert_eq!(marker_commit.change_id, change_id);
        let marker_oid = marker_commit.write()?;
        let marker_commit = repo.repo.find_commit(marker_oid)?;
        assert_eq!(
            marker_commit.parent_count(),
            1,
            "marker commit should have one parent"
        );

        let marker_parent = marker_commit.parent(0)?;
        let a_tree_id = repo.repo.find_commit(a.oid())?.tree_id();
        assert_eq!(
            marker_parent.tree_id(),
            a_tree_id,
            "marker commit's tree differs from base commit"
        );

        let ref_name = marker_commit_ref_name(&change_id);
        let marker_commit_ref = repo.repo.find_reference(&ref_name)?;
        assert_eq!(
            marker_commit_ref.peel_to_commit()?.id(),
            marker_commit.id(),
            "marker commit not stored at expected ref"
        );
        Ok(())
    }

    #[test]
    fn reuse_marker_commit() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let marker_1 = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        let sha_1 = marker_1.write()?;
        drop(marker_1);

        let marker_2 = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        let sha_2 = marker_2.write()?;
        assert_eq!(sha_1, sha_2, "marker commit not reused");
        Ok(())
    }

    #[test]
    fn reuse_root_marker() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        let a = repo.commit("commit A")?.created;
        let change_id = change_id(&a.change_id);
        let marker_1 = MarkerCommit::get(&repo.repo, &change_id, a.oid())?;
        let sha_1 = marker_1.write()?;
        drop(marker_1);

        let marker_2 = MarkerCommit::get(&repo.repo, &change_id, a.oid())?;
        let sha_2 = marker_2.write()?;
        assert_eq!(sha_1, sha_2, "marker commit not reused");
        Ok(())
    }

    #[test]
    fn create_and_clear_lock_file() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let c = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        let lock_path = MarkerCommitLock::lock_path(&repo.repo, &change_id);

        assert!(
            lock_path.exists(),
            "lock file missing while markerCommit is alive"
        );
        drop(c);
        assert!(
            !lock_path.exists(),
            "lock file not deleted after markerCommit dropped"
        );
        Ok(())
    }

    #[test]
    fn cherry_pick_when_rebased() -> Result {
        // B   R        B'  R'
        //  \ /   -->   \  /
        //   A           A'
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let r = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello again")?;
        let a_2 = repo.repo.find_commit(repo.work_copy()?.oid())?;
        repo.edit(&b.change_id)?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, &change_id, b_2.oid())?;
        let r2_oid = r2.write()?;
        let r2_commit = repo.repo.find_commit(r2_oid)?;
        assert_eq!(r2_commit.parent_count(), 1);
        assert_eq!(
            r2_commit.parent(0)?.id(),
            a_2.id(),
            "marker commit not cherry-picked onto new parent"
        );
        assert_eq!(
            r2_commit.tree_id(),
            a_2.tree_id(),
            "marker commit tree differs from new base commit"
        );
        Ok(())
    }

    #[test]
    fn initial_commit() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        let a = repo.commit("commit A")?.created;

        let change_id = change_id(&a.change_id);
        let marker_commit = MarkerCommit::get(&repo.repo, &change_id, a.oid())?;
        let marker_oid = marker_commit.write()?;
        let marker_commit = repo.repo.find_commit(marker_oid)?;

        let empty_tree_oid: Oid = empty_tree(&repo.repo)?;

        assert_eq!(
            marker_commit.parent_count(),
            0,
            "initial commit should have no parent"
        );
        assert_eq!(
            marker_commit.tree_id(),
            empty_tree_oid,
            "marker commit tree should match initial commit tree"
        );

        Ok(())
    }

    #[test]
    fn test_mutual_exclusion() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let path = repo.path().to_string();
        let active_threads = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..20 {
            let active_threads = Arc::clone(&active_threads);
            let path = path.clone();
            let b = b.clone();
            handles.push(thread::spawn(move || {
                let repo = Repository::open(path).unwrap();
                let c = MarkerCommit::get(&repo, &change_id(&b.change_id), b.oid()).unwrap();
                let current = active_threads.fetch_add(1, Ordering::SeqCst);
                assert!(
                    current == 0,
                    "concurrent access to marker commit is not allowed"
                );
                thread::sleep(Duration::from_millis(50));
                active_threads.fetch_sub(1, Ordering::SeqCst);
                drop(c);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        Ok(())
    }

    // ── mark_file_reviewed tests ────────────────────────────────────────
    #[test]
    fn state_persists_after_write() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let mut marker_1 = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;

        marker_1.mark_file_reviewed(Path::new("test2"), None)?;
        marker_1.write()?;
        drop(marker_1);

        let marker_2 = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        let un_reviewed_files = marker_2.un_reviewed_files()?;
        assert!(
            un_reviewed_files.is_empty(),
            "reviewed state should persist after write and reload"
        );
        Ok(())
    }

    #[test]
    fn mark_file_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let mut marker_commit = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;

        let un_reviewed_files = marker_commit.un_reviewed_files()?;
        assert_eq!(
            un_reviewed_files.len(),
            1,
            "no file should be marked as reviewed before marking"
        );

        marker_commit.mark_file_reviewed(Path::new("test2"), None)?;

        let un_reviewed_files = marker_commit.un_reviewed_files()?;
        assert!(
            un_reviewed_files.is_empty(),
            "all files should be marked as reviewed"
        );
        Ok(())
    }

    #[test]
    fn mark_file_reviewed_with_rename() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        repo.commit("commit A")?;
        repo.rename_file("test", "test2")?;
        let b = repo.commit("commit B")?.created;

        let mut marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b.oid())?;
        let un_reviewed_files = marker.un_reviewed_files()?;
        assert_eq!(
            un_reviewed_files.len(),
            1,
            "no file should be marked as reviewed before marking"
        );

        marker.mark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;

        let un_reviewed_files = marker.un_reviewed_files()?;
        assert!(
            un_reviewed_files.is_empty(),
            "all files should be marked as reviewed after rename"
        );

        Ok(())
    }

    #[test]
    fn mark_deleted_file_reviewed() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        repo.commit("commit A")?;
        repo.delete_file("test")?;
        let b = repo.commit("commit B")?.created;

        let mut marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test"), None)?;
        assert!(
            marker.un_reviewed_files()?.is_empty(),
            "deleted file should be marked as reviewed"
        );

        Ok(())
    }

    #[test]
    fn unmark_modified_file_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let mut marker_commit = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;

        marker_commit.mark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            marker_commit.un_reviewed_files()?.is_empty(),
            "file should be marked as reviewed"
        );
        marker_commit.unmark_file_reviewed(Path::new("test2"), None)?;

        let un_reviewed_files = marker_commit.un_reviewed_files()?;
        assert_eq!(
            un_reviewed_files.len(),
            1,
            "file should be marked as un-reviewed after un-marking"
        );
        Ok(())
    }

    #[test]
    fn unmark_added_file_reviewed() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        repo.commit("commit A")?;
        repo.write_file("test2", "hello world")?;
        let b = repo.commit("commit B")?.created;

        let mut marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            marker.un_reviewed_files()?.is_empty(),
            "added file should be marked as reviewed"
        );
        marker.unmark_file_reviewed(Path::new("test2"), None)?;

        let un_reviewed_files = marker.un_reviewed_files()?;
        assert_eq!(
            un_reviewed_files.len(),
            1,
            "added file should be marked as un-reviewed after un-marking"
        );

        Ok(())
    }

    #[test]
    fn unmark_renamed_file_reviewed() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello")?;
        repo.commit("commit A")?;
        repo.rename_file("test", "test2")?;
        let b = repo.commit("commit B")?.created;

        let mut marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;
        assert!(
            marker.un_reviewed_files()?.is_empty(),
            "renamed file should be marked as reviewed"
        );
        marker.unmark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;
        assert!(
            marker.un_reviewed_files()?.contains(Path::new("test2")),
            "renamed file should be marked as un-reviewed after un-marking"
        );

        Ok(())
    }

    #[test]
    fn survive_rewriting_unrelated_file() -> Result {
        // B   R        B'  R'
        //  \ /   -->   \  /
        //   A           A'
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let mut r = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello again")?;
        repo.edit(&b.change_id)?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, &change_id, b_2.oid())?;
        let un_reviewed_files = r2.un_reviewed_files()?;
        assert!(
            un_reviewed_files.is_empty(),
            "reviewed state should survive non-conflicting rebase"
        );
        Ok(())
    }

    #[test]
    fn survive_rewriting_unrelated_region_of_file() -> Result {
        // B   R        B'  R'
        //  \ /   -->   \  /
        //   A           A'
        let repo = TestRepo::new()?;
        repo.write_file("test", "hello\nworld\nwill_be_modified\n")?;
        let a = repo.commit("commit A")?.created;
        repo.write_file("test", "hello\nworld\nmodified\n")?;
        let b = repo.commit("commit B")?.created;
        let change_id_b = change_id(&b.change_id);

        let mut marker = MarkerCommit::get(&repo.repo, &change_id_b, b.oid())?;
        marker.mark_file_reviewed(Path::new("test"), None)?;
        marker.write()?;
        drop(marker);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello_2\nworld\nwill_be_modified\n")?;
        repo.edit(&b.change_id)?;

        let r = MarkerCommit::get(&repo.repo, &change_id_b, b.oid())?;
        assert!(
            r.un_reviewed_files()?.is_empty(),
            "reviewed state should survive non-conflicting rebase even if the file content is modified"
        );

        Ok(())
    }

    #[test]
    fn changing_diff_revert_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let mut r = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&b.change_id)?;
        repo.write_file("test2", "hello again")?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, &change_id, b_2.oid())?;
        let un_reviewed_files = r2.un_reviewed_files()?;
        assert!(
            un_reviewed_files.contains(Path::new("test2")),
            "reviewed state should be reverted if the file content is changed in a conflicting way"
        );
        Ok(())
    }

    // ─────rebase conflict tests ───────────────────────────────────────

    #[test]
    fn take_base_when_conflict() -> Result {
        // B   R       B'   R'
        //  \ /   -->   \  /
        //   A           A'
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let mut r = MarkerCommit::get(&repo.repo, &change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test2", "hello again")?;
        let a_2 = repo.repo.find_commit(repo.work_copy()?.oid())?;
        repo.edit(&b.change_id)?;
        repo.write_file("test2", "hello fixed")?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, &change_id, b_2.oid())?;
        let r2_oid = r2.write()?;
        let r2_commit = repo.repo.find_commit(r2_oid)?;
        assert_eq!(r2_commit.parent_count(), 1);
        assert_eq!(
            r2_commit.parent(0)?.id(),
            a_2.id(),
            "marker commit should take new base as parent even when there is conflict"
        );
        assert_eq!(
            r2_commit.tree_id(),
            a_2.tree_id(),
            "marker commit tree should be same as new base even when there is conflict"
        );
        Ok(())
    }

    #[test]
    fn only_invalidate_conflicted_file() -> Result {
        // B   R       B'   R'
        //  \ /   -->   \  /
        //   A           A'

        let repo = TestRepo::new()?;
        repo.write_file("test", "hello\n")?;
        let a = repo.commit("commit A")?.created;
        repo.write_file("test2", "hello\n")?;
        repo.write_file("test3", "hello\n")?;
        repo.write_file("test", "hello again\n")?;
        let b = repo.commit("commit B")?.created;

        let mut marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test"), None)?;
        marker.mark_file_reviewed(Path::new("test2"), None)?;
        marker.mark_file_reviewed(Path::new("test3"), None)?;
        marker.write()?;
        drop(marker);

        // edit a into a2
        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello again again\n")?;
        let _a_2 = repo.work_copy()?;

        repo.edit(&b.change_id)?;
        repo.write_file("test", "hello fixed\n")?;
        let b_2 = repo.work_copy()?;

        let marker = MarkerCommit::get(&repo.repo, &change_id(&b.change_id), b_2.oid())?;
        let un_reviewed_files = marker.un_reviewed_files()?;
        assert_eq!(
            un_reviewed_files.len(),
            1,
            "only the file with conflict should be marked as un-reviewed"
        );
        assert!(
            un_reviewed_files.contains(Path::new("test")),
            "the file with conflict should be marked as un-reviewed"
        );

        let test_file_oid = marker.tree.get_name("test").unwrap().id();
        let test_file_blob = repo.repo.find_blob(test_file_oid)?;
        let test_file_content = std::str::from_utf8(test_file_blob.content())?;
        assert_eq!(
            test_file_content, "hello again again\n",
            "the content of conflicted file in marker commit should be same as the new base"
        );

        Ok(())
    }
}
