use std::process::Command;

use git2::{IndexAddOption, Repository};
use tempfile::TempDir;

pub struct TestRepo {
    _dir: TempDir,
    pub repo: Repository,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        Self { _dir: dir, repo }
    }

    pub fn path(&self) -> &str {
        self._dir.path().to_str().unwrap()
    }

    pub fn write_file(&self, path: &str, content: &str) {
        let file_path = self._dir.path().join(path);
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, content).unwrap();
    }

    pub fn delete_file(&self, path: &str) {
        let file_path = self._dir.path().join(path);
        std::fs::remove_file(&file_path).unwrap();
    }

    pub fn rename_file(&self, old_path: &str, new_path: &str) {
        let old_file_path = self._dir.path().join(old_path);
        let new_file_path = self._dir.path().join(new_path);
        std::fs::create_dir_all(new_file_path.parent().unwrap()).unwrap();
        std::fs::rename(&old_file_path, &new_file_path).unwrap();
    }

    // Add all changes and commit with the given message returning the new commit's SHA
    pub fn commit(&self, message: &str) -> git2::Oid {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = self.repo.find_tree(tree_id).unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let parent = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    /// Create a merge commit with multiple parents using current working tree state.
    /// Call write_file() to set up the tree before calling this.
    pub fn commit_with_parents(&self, parent_shas: &[git2::Oid], message: &str) -> git2::Oid {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = self.repo.find_tree(tree_id).unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let parents: Vec<git2::Commit> = parent_shas
            .iter()
            .map(|sha| self.repo.find_commit(sha.clone()).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        self.repo
            .commit(None, &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    pub fn setup_jujutu(&self) {
        let output = Command::new("jj")
            .args(["git", "init"])
            .current_dir(&self._dir)
            .output()
            .expect("jj must be installed to run these tests");
        assert!(
            output.status.success(),
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Configure user identity for the repo
        Command::new("jj")
            .args(["config", "set", "--repo", "user.name", "Test User"])
            .current_dir(&self._dir)
            .output()
            .unwrap();
        Command::new("jj")
            .args(["config", "set", "--repo", "user.email", "test@test.com"])
            .current_dir(&self._dir)
            .output()
            .unwrap();
    }
}
