use crate::{
    ChangeId, Error, HunkId, Result,
    apply_hunk::{apply_hunk, unapply_hunk},
    conflict::resolve_conflict_prefer_our,
    marker_commit_lock::MarkerCommitLock,
    octopus_merge::octopus_merge,
    tree_builder_ext::TreeBuilderExt,
};
use git2::{Commit, Oid, Repository, Signature, Tree};
use std::path::Path;

/// Commit for tracking review state for a specific revision.
/// Stored at refs/kenjutu/{change_id}/marker pointing to the commit being reviewed.
pub struct MarkerCommit<'a> {
    change_id: ChangeId,
    tree: Tree<'a>,
    target: Commit<'a>,
    base_tree: Tree<'a>,
    repo: &'a Repository,
    _guard: MarkerCommitLock,
}

impl<'a> std::fmt::Debug for MarkerCommit<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkerCommit")
            .field("change_id", &self.change_id)
            .field("target", &self.target)
            .finish()
    }
}

impl<'a> MarkerCommit<'a> {
    pub fn get(repo: &'a Repository, change_id: impl Into<ChangeId>, sha: Oid) -> Result<Self> {
        let change_id = change_id.into();
        let lock_file = MarkerCommitLock::new(repo, change_id)?;
        log::info!(
            "acquired lock for marker commit for revision: {}",
            change_id
        );
        let target_commit = repo.find_commit(sha)?;

        let new_base_tree = calculate_base_tree(repo, &target_commit)?;

        let ref_name = marker_commit_ref_name(change_id);
        let marker_tree = match repo.find_reference(&ref_name) {
            Ok(reference) => {
                let marker_commit = reference.peel_to_commit()?;
                // Marker commits must have a single parent which is the target commit.
                let old_target_commit = if marker_commit.parent_count() == 1 {
                    marker_commit.parent(0)?
                } else {
                    return Err(Error::MarkerCommitNonOneParent {
                        change_id,
                        parent_count: marker_commit.parent_count(),
                        marker_commit_id: marker_commit.id(),
                    });
                };

                let old_base_tree = calculate_base_tree(repo, &old_target_commit)?;
                if old_base_tree.id() == new_base_tree.id() {
                    marker_commit.tree()?
                } else {
                    let mut index = repo.merge_trees(
                        &old_base_tree,
                        &new_base_tree,
                        &marker_commit.tree()?,
                        None,
                    )?;
                    if index.has_conflicts() {
                        let resolved_tree_oid = resolve_conflict_prefer_our(repo, &mut index)?;
                        repo.find_tree(resolved_tree_oid)?
                    } else {
                        repo.find_tree(index.write_tree_to(repo)?)?
                    }
                }
            }
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    return Err(Error::Git(err));
                }
                new_base_tree.clone()
            }
        };

        Ok(Self {
            _guard: lock_file,
            tree: marker_tree,
            base_tree: new_base_tree,
            target: target_commit,
            repo,
            change_id,
        })
    }

    pub fn marker_tree(&self) -> &Tree<'a> {
        &self.tree
    }

    pub fn base_tree(&self) -> &Tree<'a> {
        &self.base_tree
    }

    /// Mark a single hunk as reviewed by splicing the corresponding target lines into the marker blob.
    ///
    /// `hunk` coordinates must be in M/T space, as they appear in `diff(marker, target)`.
    ///
    /// For renamed files, always supply `old_path` (the file's name in the base commit).
    /// On the first hunk mark the file is still at `old_path` in M, so the blob is moved to
    /// `file_path`. On subsequent marks M already has the file at `file_path`, so the lookup
    /// falls back automatically — the caller does not need to track this.
    pub fn mark_hunk_reviewed(
        &mut self,
        file_path: &Path,
        old_path: Option<&Path>,
        hunk: &HunkId,
    ) -> Result<()> {
        let ext = TreeBuilderExt::new(self.repo);

        // Determine where the blob currently lives in M.
        // If old_path is given and still present in M the rename hasn't been applied yet.
        // If old_path is absent (already moved to file_path by a previous hunk mark) fall back.
        let (m_lookup, rename_pending) = if let Some(op) = old_path {
            match self.tree.get_path(op) {
                Ok(_) => (op, true),
                Err(e) if e.code() == git2::ErrorCode::NotFound => (file_path, false),
                Err(e) => return Err(Error::Git(e)),
            }
        } else {
            (file_path, false)
        };

        let (m_content, m_filemode) = blob_content_and_mode(&self.tree, m_lookup, self.repo)?;
        let (t_content, _) = blob_content_and_mode(&self.target.tree()?, file_path, self.repo)?;

        let new_content = apply_hunk(&m_content, &t_content, hunk);
        let new_oid = self.repo.blob(new_content.as_bytes())?;

        if rename_pending {
            let tree_oid = ext.remove_path(&self.tree, m_lookup)?;
            let tree = self.repo.find_tree(tree_oid)?;
            let new_tree_oid = ext.insert_file(&tree, file_path, new_oid, m_filemode)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
        } else {
            let new_tree_oid = ext.insert_file(&self.tree, file_path, new_oid, m_filemode)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
        }
        Ok(())
    }

    /// Unmark a single hunk as reviewed by splicing the base lines back into the marker blob.
    ///
    /// `hunk` coordinates must be in B/M space, as they appear in `diff(base, marker)`:
    /// `old_*` are base coordinates, `new_*` are marker coordinates.
    ///
    /// For renamed files, always supply `old_path` (the file's name in the base commit) so the
    /// correct base content can be restored. The blob in M is always looked up and written back
    /// at `file_path` — unmarking a hunk reverts content only, not the rename in M.
    pub fn unmark_hunk_reviewed(
        &mut self,
        file_path: &Path,
        old_path: Option<&Path>,
        hunk: &HunkId,
    ) -> Result<()> {
        let ext = TreeBuilderExt::new(self.repo);

        let (m_content, m_filemode) = blob_content_and_mode(&self.tree, file_path, self.repo)?;

        let b_lookup = old_path.unwrap_or(file_path);
        let (b_content, file_in_base) = {
            match self.base_tree.get_path(b_lookup) {
                Ok(entry) => {
                    let blob = self.repo.find_blob(entry.id())?;
                    let content = std::str::from_utf8(blob.content())
                        .map_err(|e| Error::Internal(e.to_string()))?
                        .to_owned();
                    (content, true)
                }
                Err(e) if e.code() == git2::ErrorCode::NotFound => (String::new(), false),
                Err(e) => return Err(Error::Git(e)),
            }
        };

        let new_content = unapply_hunk(&m_content, &b_content, hunk);
        if new_content.is_empty() && !file_in_base {
            let new_tree_oid = ext.remove_path(&self.tree, file_path)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
        } else {
            let new_oid = self.repo.blob(new_content.as_bytes())?;
            let new_tree_oid = ext.insert_file(&self.tree, file_path, new_oid, m_filemode)?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
        }
        Ok(())
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
            let new_file = self.target.tree()?.get_path(file_path)?;
            let tree_after_remove = ext.remove_path(&self.tree, old_path)?;
            let tree = self.repo.find_tree(tree_after_remove)?;
            let new_tree_oid =
                ext.insert_file(&tree, file_path, new_file.id(), new_file.filemode())?;
            self.tree = self.repo.find_tree(new_tree_oid)?;
            return Ok(());
        }

        match self.target.tree()?.get_path(file_path) {
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
            let old_content = self.base_tree.get_path(old_path)?;
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

        match self.base_tree.get_path(file_path) {
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

    /// Write the review status to the repository. Should be called after marking files as
    /// reviewed.
    /// Return the`Oid` of the marker commit.
    pub fn write(&self) -> Result<Oid> {
        let message = format!("update marker commit for change_id: {}", self.change_id);
        let signature = Self::signature()?;
        let oid = self.repo.commit(
            None,
            &signature,
            &signature,
            &message,
            &self.tree,
            &[&self.target],
        )?;
        log::info!("created marker commit for {}", self.change_id);

        let ref_name = marker_commit_ref_name(self.change_id);
        log::info!("Updating ref: {}", &ref_name);
        let log_message = format!(
            "kenjutu: updated reference for marker commit for change_id: {}",
            self.change_id
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

fn calculate_base_tree<'a>(repo: &'a Repository, commit: &Commit<'a>) -> Result<Tree<'a>> {
    match commit.parent_count() {
        0 => {
            let empty_tree_oid = empty_tree(repo)?;
            let tree = repo.find_tree(empty_tree_oid)?;
            Ok(tree)
        }
        1 => Ok(commit.parent(0)?.tree()?),
        _ => {
            let parents = commit.parents().collect::<Vec<_>>();
            let merged_bases_oid =
                octopus_merge(repo, &parents)?.ok_or_else(|| Error::BasesMergeConflict {
                    commit_id: commit.id(),
                })?;
            Ok(repo.find_tree(merged_bases_oid)?)
        }
    }
}

fn empty_tree(repo: &Repository) -> Result<Oid> {
    let builder = repo.treebuilder(None)?;
    let oid = builder.write()?;
    Ok(oid)
}

/// Look up a blob at `path` in `tree`, returning its content as a `String` and its filemode.
fn blob_content_and_mode(tree: &Tree<'_>, path: &Path, repo: &Repository) -> Result<(String, i32)> {
    let entry = tree.get_path(path)?;
    let filemode = entry.filemode();
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|e| Error::Internal(e.to_string()))?
        .to_owned();
    Ok((content, filemode))
}

fn marker_commit_ref_name(change_id: ChangeId) -> String {
    format!("refs/kenjutu/{}/marker", change_id)
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
        ChangeId::try_from(s).unwrap()
    }

    /// Returns `true` when `file_path` has the same blob OID in the marker tree and the target
    /// tree (both absent counts as equal — the file was deleted and that deletion is reviewed).
    fn does_oid_match(marker: &MarkerCommit, file_path: &Path) -> bool {
        let target_tree = marker.target.tree().unwrap();
        let m_id = marker.tree.get_path(file_path).ok().map(|e| e.id());
        let t_id = target_tree.get_path(file_path).ok().map(|e| e.id());
        m_id == t_id
    }

    // ── MarkerCommit::get tests ────────────────────────────────────────

    #[test]
    fn create_marker_commit() -> Result {
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let marker_commit = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        marker_commit.write()?;

        assert_eq!(marker_commit.change_id, change_id);
        let marker_oid = marker_commit.write()?;
        let marker_commit = repo.repo.find_commit(marker_oid)?;
        assert_eq!(
            marker_commit.parent_count(),
            1,
            "marker commit should have one parent"
        );

        let a_tree_id = repo.repo.find_commit(a.oid())?.tree_id();
        assert_eq!(
            marker_commit.tree_id(),
            a_tree_id,
            "marker commit's tree differs from base commit"
        );

        let ref_name = marker_commit_ref_name(change_id);
        let marker_commit_ref = repo.repo.find_reference(&ref_name)?;
        assert_eq!(
            marker_commit_ref.peel_to_commit()?.id(),
            marker_commit.id(),
            "marker commit not stored at expected ref"
        );
        Ok(())
    }

    #[test]
    fn create_and_clear_lock_file() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let c = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        let lock_path = MarkerCommitLock::lock_path(&repo.repo, change_id);

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
        // B -- R      B' -- R'
        //  \    -->   \
        //   A          A'
        let (repo, a, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let r = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello again")?;
        let a_2 = repo.repo.find_commit(repo.work_copy()?.oid())?;
        repo.edit(&b.change_id)?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, change_id, b_2.oid())?;
        let r2_oid = r2.write()?;
        let r2_commit = repo.repo.find_commit(r2_oid)?;
        assert_eq!(r2_commit.parent_count(), 1);
        assert_eq!(
            r2_commit.parent(0)?.id(),
            b_2.oid(),
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
        let marker_commit = MarkerCommit::get(&repo.repo, change_id, a.oid())?;
        let marker_oid = marker_commit.write()?;
        let marker_commit = repo.repo.find_commit(marker_oid)?;

        let empty_tree_oid: Oid = empty_tree(&repo.repo)?;

        assert_eq!(
            marker_commit.parent_count(),
            1,
            "marker commit should take target commit as parent"
        );
        assert_eq!(
            marker_commit.parent(0)?.id(),
            a.oid(),
            "marker commit parent should be the initial commit"
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
                let c = MarkerCommit::get(&repo, change_id(&b.change_id), b.oid()).unwrap();
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
        let mut marker_1 = MarkerCommit::get(&repo.repo, change_id, b.oid())?;

        marker_1.mark_file_reviewed(Path::new("test2"), None)?;
        let m1_tree_oid = marker_1.tree.id();
        marker_1.write()?;
        drop(marker_1);

        let marker_2 = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        let marker_tree_oid = marker_2.tree.id();
        assert_eq!(
            marker_tree_oid, m1_tree_oid,
            "reviewed state should persist after write and reload"
        );
        Ok(())
    }

    #[test]
    fn mark_file_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let mut marker_commit = MarkerCommit::get(&repo.repo, change_id, b.oid())?;

        assert!(
            !does_oid_match(&marker_commit, Path::new("test2")),
            "test2 should not be reviewed before marking"
        );

        marker_commit.mark_file_reviewed(Path::new("test2"), None)?;

        assert!(
            does_oid_match(&marker_commit, Path::new("test2")),
            "test2 should be reviewed after marking"
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b.oid())?;
        assert!(
            !does_oid_match(&marker, Path::new("test2")),
            "test2 should not be reviewed before marking"
        );

        marker.mark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;

        assert!(
            does_oid_match(&marker, Path::new("test2")),
            "test2 should be reviewed after rename mark"
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test"), None)?;
        assert!(
            does_oid_match(&marker, Path::new("test")),
            "deleted file should be reviewed (both absent in marker and target)"
        );

        Ok(())
    }

    #[test]
    fn unmark_modified_file_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);
        let mut marker_commit = MarkerCommit::get(&repo.repo, change_id, b.oid())?;

        marker_commit.mark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            does_oid_match(&marker_commit, Path::new("test2")),
            "test2 should match target after marking"
        );
        marker_commit.unmark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            !does_oid_match(&marker_commit, Path::new("test2")),
            "test2 should not match target after un-marking"
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            does_oid_match(&marker, Path::new("test2")),
            "added file should match target after marking"
        );
        marker.unmark_file_reviewed(Path::new("test2"), None)?;
        assert!(
            !does_oid_match(&marker, Path::new("test2")),
            "added file should not match target after un-marking"
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b.oid())?;
        marker.mark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;
        assert!(
            does_oid_match(&marker, Path::new("test2")),
            "renamed file should match target after marking"
        );
        marker.unmark_file_reviewed(Path::new("test2"), Some(Path::new("test")))?;
        assert!(
            !does_oid_match(&marker, Path::new("test2")),
            "renamed file should not match target after un-marking"
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

        let mut r = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello again")?;
        repo.edit(&b.change_id)?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, change_id, b_2.oid())?;
        assert!(
            does_oid_match(&r2, Path::new("test2")),
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id_b, b.oid())?;
        marker.mark_file_reviewed(Path::new("test"), None)?;
        marker.write()?;
        drop(marker);

        repo.edit(&a.change_id)?;
        repo.write_file("test", "hello_2\nworld\nwill_be_modified\n")?;
        repo.edit(&b.change_id)?;

        let r = MarkerCommit::get(&repo.repo, change_id_b, b.oid())?;
        assert!(
            does_oid_match(&r, Path::new("test")),
            "reviewed state should survive non-conflicting rebase even if the file content is modified"
        );

        Ok(())
    }

    #[test]
    fn changing_diff_revert_reviewed() -> Result {
        let (repo, _, b) = setup_two_commits()?;
        let change_id = change_id(&b.change_id);

        let mut r = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&b.change_id)?;
        repo.write_file("test2", "hello again")?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, change_id, b_2.oid())?;
        assert!(
            !does_oid_match(&r2, Path::new("test2")),
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

        let mut r = MarkerCommit::get(&repo.repo, change_id, b.oid())?;
        r.mark_file_reviewed(Path::new("test2"), None)?;
        r.write()?;
        drop(r);

        repo.edit(&a.change_id)?;
        repo.write_file("test2", "hello again")?;
        let a_2 = repo.repo.find_commit(repo.work_copy()?.oid())?;
        repo.edit(&b.change_id)?;
        repo.write_file("test2", "hello fixed")?;
        let b_2 = repo.work_copy()?;

        let r2 = MarkerCommit::get(&repo.repo, change_id, b_2.oid())?;
        let r2_oid = r2.write()?;
        let r2_commit = repo.repo.find_commit(r2_oid)?;
        assert_eq!(r2_commit.parent_count(), 1);
        assert_eq!(
            r2_commit.parent(0)?.id(),
            b_2.oid(),
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

        let mut marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b.oid())?;
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

        let marker = MarkerCommit::get(&repo.repo, change_id(&b.change_id), b_2.oid())?;
        assert!(
            !does_oid_match(&marker, Path::new("test")),
            "the conflicted file should not match target"
        );
        assert!(
            does_oid_match(&marker, Path::new("test2")),
            "the non-conflicted file test2 should still match target"
        );
        assert!(
            does_oid_match(&marker, Path::new("test3")),
            "the non-conflicted file test3 should still match target"
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

    // ── mark_hunk_reviewed / unmark_hunk_reviewed tests ───────────────

    /// Build a two-hunk file: base has "a"s and "b"s; target changes one "a" and one "b".
    ///
    /// Base ("test"):
    ///   a1 / a2 / a3 / a4 / a5 / b1 / b2 / b3 / b4 / b5
    /// Target ("test"):
    ///   A1 / a2 / a3 / a4 / a5 / b1 / b2 / b3 / B4 / b5
    ///
    /// diff(base→target) has two hunks:
    ///   hunk1: @@ -1,3 +1,3 @@ (context a2, changed a1→A1, context a3… well, 3-line window)
    ///   hunk2: @@ -8,3 +8,3 @@ (context b3, changed b4→B4, context b5)
    fn setup_two_hunk_commit() -> Result<(TestRepo, String, String, HunkId, HunkId)> {
        let repo = TestRepo::new()?;
        let base_content = "a1\na2\na3\na4\na5\nb1\nb2\nb3\nb4\nb5\n";
        let target_content = "A1\na2\na3\na4\na5\nb1\nb2\nb3\nB4\nb5\n";
        repo.write_file("test", base_content)?;
        let _a = repo.commit("commit A")?.created;
        repo.write_file("test", target_content)?;
        let b = repo.commit("commit B")?.created;
        // Hunk1: @@ -1,3 +1,3 @@ — lines 1-3 (a1→A1 with context a2, a3)
        let hunk1 = HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        // Hunk2: @@ -8,3 +8,3 @@ — lines 8-10 (b4→B4 with context b3, b5)
        let hunk2 = HunkId {
            old_start: 8,
            old_lines: 3,
            new_start: 8,
            new_lines: 3,
        };
        Ok((repo, b.change_id, b.commit_id, hunk1, hunk2))
    }

    fn blob_content_at(repo: &git2::Repository, tree: &git2::Tree, path: &Path) -> String {
        let entry = tree.get_path(path).unwrap();
        let blob = repo.find_blob(entry.id()).unwrap();
        std::str::from_utf8(blob.content()).unwrap().to_owned()
    }

    #[test]
    fn mark_first_hunk_leaves_second_unreviewed() -> Result {
        let (repo, change_id_str, sha, hunk1, _hunk2) = setup_two_hunk_commit()?;
        let sha_oid = git2::Oid::from_str(&sha).unwrap();
        let change_id = change_id(&change_id_str);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        marker.mark_hunk_reviewed(Path::new("test"), None, &hunk1)?;

        // hunk1 region (line 1) should now match target; hunk2 region (line 9) should not
        let m_content = blob_content_at(&repo.repo, &marker.tree, Path::new("test"));
        let lines: Vec<&str> = m_content.lines().collect();
        assert_eq!(lines[0], "A1", "hunk1 should be applied");
        assert_eq!(lines[8], "b4", "hunk2 should still be base content");
        Ok(())
    }

    #[test]
    fn mark_all_hunks_makes_file_reviewed() -> Result {
        let (repo, change_id_str, sha, hunk1, _hunk2) = setup_two_hunk_commit()?;
        let sha_oid = git2::Oid::from_str(&sha).unwrap();
        let change_id = change_id(&change_id_str);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        // After marking hunk1, M changes so hunk2 coords shift; but our two hunks are
        // far enough apart that the M/T coords are identical to the original B/T coords.
        marker.mark_hunk_reviewed(Path::new("test"), None, &hunk1)?;
        // Re-derive hunk2 coords in M/T space (same as B/T since only line 1 changed)
        let hunk2_in_mt = HunkId {
            old_start: 8,
            old_lines: 3,
            new_start: 8,
            new_lines: 3,
        };
        marker.mark_hunk_reviewed(Path::new("test"), None, &hunk2_in_mt)?;

        let target_content = "A1\na2\na3\na4\na5\nb1\nb2\nb3\nB4\nb5\n";
        let m_content = blob_content_at(&repo.repo, &marker.tree, Path::new("test"));
        assert_eq!(
            m_content, target_content,
            "all hunks marked → M should equal T"
        );
        Ok(())
    }

    #[test]
    fn unmark_hunk_reverts_to_base() -> Result {
        let (repo, change_id_str, sha, hunk1, _hunk2) = setup_two_hunk_commit()?;
        let sha_oid = git2::Oid::from_str(&sha).unwrap();
        let change_id = change_id(&change_id_str);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        // Mark hunk1; now diff(B→M) has hunk1 with same coords as hunk1 in diff(B→T)
        marker.mark_hunk_reviewed(Path::new("test"), None, &hunk1)?;

        let m_after_mark = blob_content_at(&repo.repo, &marker.tree, Path::new("test"));
        assert_eq!(m_after_mark.lines().next().unwrap(), "A1");

        // Unmark using B/M coords (same as hunk1 since only that region changed)
        marker.unmark_hunk_reviewed(Path::new("test"), None, &hunk1)?;

        let base_content = "a1\na2\na3\na4\na5\nb1\nb2\nb3\nb4\nb5\n";
        let m_after_unmark = blob_content_at(&repo.repo, &marker.tree, Path::new("test"));
        assert_eq!(
            m_after_unmark, base_content,
            "unmark should restore base content"
        );
        Ok(())
    }

    // ── rename + hunk tests ───────────────────────────────────────────
    //
    // Setup:
    //   Base  "old.txt": head / a1 / mid1 / mid2 / mid3 / b1 / tail
    //   Target "new.txt": head / A1 / mid1 / mid2 / mid3 / B1 / tail  (renamed + two hunks)
    //
    // M starts at base tree: has "old.txt" at base content.
    // After mark_hunk_reviewed(new.txt, Some(old.txt), hunk1):
    //   → M no longer has "old.txt"; now has "new.txt" with hunk1 applied.
    // After mark_hunk_reviewed(new.txt, None, hunk2):
    //   → "new.txt" in M equals target content.
    //
    // hunk1 (M/T space, initial M == base):  @@ -1,3 +1,3 @@
    // hunk2 (M/T space, after hunk1 applied): @@ -5,3 +5,3 @@ (coords unchanged: same line count)

    fn setup_rename_two_hunk_commit() -> Result<(TestRepo, String, String, HunkId, HunkId)> {
        let repo = TestRepo::new()?;
        let base_content = "head\na1\nmid1\nmid2\nmid3\nb1\ntail\n";
        let target_content = "head\nA1\nmid1\nmid2\nmid3\nB1\ntail\n";
        repo.write_file("old.txt", base_content)?;
        let _a = repo.commit("commit A")?.created;
        repo.rename_file("old.txt", "new.txt")?;
        repo.write_file("new.txt", target_content)?;
        let b = repo.commit("commit B")?.created;
        let hunk1 = HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        let hunk2 = HunkId {
            old_start: 5,
            old_lines: 3,
            new_start: 5,
            new_lines: 3,
        };
        Ok((repo, b.change_id, b.commit_id, hunk1, hunk2))
    }

    #[test]
    fn mark_first_hunk_of_renamed_file_moves_blob_to_new_path() -> Result {
        let (repo, change_id_str, sha, hunk1, _hunk2) = setup_rename_two_hunk_commit()?;
        let sha_oid = git2::Oid::from_str(&sha).unwrap();
        let change_id = change_id(&change_id_str);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        marker.mark_hunk_reviewed(Path::new("new.txt"), Some(Path::new("old.txt")), &hunk1)?;

        // old.txt must be gone from M; new.txt must exist with hunk1 applied
        assert!(
            marker.tree.get_path(Path::new("old.txt")).is_err(),
            "old.txt should be removed from M after first hunk mark"
        );
        let m_content = blob_content_at(&repo.repo, &marker.tree, Path::new("new.txt"));
        let lines: Vec<&str> = m_content.lines().collect();
        assert_eq!(lines[1], "A1", "hunk1 applied: line 2 should be A1");
        assert_eq!(
            lines[5], "b1",
            "hunk2 not yet applied: line 6 should remain b1"
        );
        Ok(())
    }

    #[test]
    fn mark_both_hunks_of_renamed_file_sequentially() -> Result {
        let (repo, change_id_str, sha, hunk1, hunk2) = setup_rename_two_hunk_commit()?;
        let sha_oid = git2::Oid::from_str(&sha).unwrap();
        let change_id = change_id(&change_id_str);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;

        // Both calls always supply old_path; the implementation detects whether the
        // rename has already been applied to M and falls back automatically.
        marker.mark_hunk_reviewed(Path::new("new.txt"), Some(Path::new("old.txt")), &hunk1)?;
        marker.mark_hunk_reviewed(Path::new("new.txt"), Some(Path::new("old.txt")), &hunk2)?;

        let target_content = "head\nA1\nmid1\nmid2\nmid3\nB1\ntail\n";
        let m_content = blob_content_at(&repo.repo, &marker.tree, Path::new("new.txt"));
        assert_eq!(
            m_content, target_content,
            "both hunks marked → M should equal T"
        );
        assert!(
            marker.tree.get_path(Path::new("old.txt")).is_err(),
            "old.txt should not be in M after full review"
        );
        Ok(())
    }

    #[test]
    fn mark_pure_addition_hunk() -> Result {
        // Base has 3 lines; target inserts a new line after line 2.
        let repo = TestRepo::new()?;
        repo.write_file("test", "line1\nline2\nline3\n")?;
        let _a = repo.commit("commit A")?.created;
        repo.write_file("test", "line1\nline2\nnew\nline3\n")?;
        let b = repo.commit("commit B")?.created;
        let sha_oid = git2::Oid::from_str(&b.commit_id).unwrap();
        let change_id = change_id(&b.change_id);

        // diff(M→T): @@ -2,0 +3,1 @@ (old_lines=0 → pure addition after line 2)
        let hunk = HunkId {
            old_start: 2,
            old_lines: 0,
            new_start: 3,
            new_lines: 1,
        };
        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        marker.mark_hunk_reviewed(Path::new("test"), None, &hunk)?;

        let m_content = blob_content_at(&repo.repo, &marker.tree, Path::new("test"));
        assert_eq!(m_content, "line1\nline2\nnew\nline3\n");
        Ok(())
    }

    #[test]
    fn unmark_all_hunks_of_added_file_removes_file_from_tree() -> Result {
        // Commit A has no "added.txt"; commit B adds it with 2 lines.
        // After marking at file level, M has the full content.
        // Unmarking the sole addition hunk should empty the content → file removed from M.
        let repo = TestRepo::new()?;
        repo.write_file("base.txt", "unchanged\n")?;
        let _a = repo.commit("commit A")?.created;
        repo.write_file("added.txt", "line1\nline2\n")?;
        let b = repo.commit("commit B")?.created;
        let sha_oid = git2::Oid::from_str(&b.commit_id).unwrap();
        let change_id = change_id(&b.change_id);

        let mut marker = MarkerCommit::get(&repo.repo, change_id, sha_oid)?;
        marker.mark_file_reviewed(Path::new("added.txt"), None)?;
        assert!(
            marker.tree.get_path(Path::new("added.txt")).is_ok(),
            "added.txt should be in M after file-level mark"
        );

        // diff(A→M): @@ -0,0 +1,2 @@ — M added both lines, A has nothing
        let hunk = HunkId {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 2,
        };
        marker.unmark_hunk_reviewed(Path::new("added.txt"), None, &hunk)?;

        assert!(
            marker.tree.get_path(Path::new("added.txt")).is_err(),
            "added.txt should be removed from M after unmarking all hunks (base had no such file)"
        );
        Ok(())
    }
}
