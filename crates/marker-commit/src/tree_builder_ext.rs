use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// A helper extension for git2 TreeBuilder that properly handles nested paths.
/// Git2's TreeBuilder only works with single path components. For nested paths
/// like "src/foo/bar.rs", we need to recursively traverse and update subtrees.
pub struct TreeBuilderExt<'repo> {
    repo: &'repo git2::Repository,
}

impl<'repo> TreeBuilderExt<'repo> {
    pub fn new(repo: &'repo git2::Repository) -> Self {
        Self { repo }
    }

    /// Insert or update a file at a nested path in a tree.
    /// This will create intermediate directories as needed.
    pub fn insert_file(
        &self,
        root_tree: &git2::Tree<'repo>,
        file_path: &Path,
        blob_oid: git2::Oid,
        filemode: i32,
    ) -> Result<git2::Oid> {
        let mut components: Vec<PathBuf> = file_path
            .components()
            .map(|c| PathBuf::from(c.as_os_str()))
            .collect();

        if components.is_empty() {
            return Err(Error::Git(git2::Error::new(
                git2::ErrorCode::GenericError,
                git2::ErrorClass::Tree,
                "empty path",
            )));
        }

        let filename = components.pop().unwrap();
        let new_tree_oid =
            self.upsert_path(root_tree, &components, 0, &filename, blob_oid, filemode)?;
        Ok(new_tree_oid)
    }

    /// Remove a file or directory at a nested path from a tree.
    pub fn remove_path(
        &self,
        root_tree: &git2::Tree<'repo>,
        file_path: &Path,
    ) -> Result<git2::Oid> {
        let components: Vec<PathBuf> = file_path
            .components()
            .map(|c| PathBuf::from(c.as_os_str()))
            .collect();

        if components.is_empty() {
            return Err(Error::Git(git2::Error::new(
                git2::ErrorCode::GenericError,
                git2::ErrorClass::Tree,
                "empty path",
            )));
        }

        let new_tree_oid = self.remove_from_path(root_tree, &components, 0)?;
        Ok(new_tree_oid)
    }

    fn upsert_path(
        &self,
        tree: &git2::Tree<'repo>,
        components: &[PathBuf],
        depth: usize,
        filename: &Path,
        blob_oid: git2::Oid,
        filemode: i32,
    ) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(Some(tree))?;

        if depth >= components.len() {
            // We're at the target directory - insert the file
            builder.insert(filename, blob_oid, filemode)?;
        } else {
            // We need to go deeper - find or create the subdirectory
            let component = &components[depth];
            let subtree = match tree.get_path(component) {
                Ok(entry) => self.repo.find_tree(entry.id())?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    // Create empty subtree
                    let empty_oid = self.repo.treebuilder(None)?.write()?;
                    self.repo.find_tree(empty_oid)?
                }
                Err(e) => return Err(Error::Git(e)),
            };

            // Recursively update the subtree
            let new_subtree_oid = self.upsert_path(
                &subtree,
                components,
                depth + 1,
                filename,
                blob_oid,
                filemode,
            )?;

            // Update the entry in the current tree
            builder.insert(component, new_subtree_oid, git2::FileMode::Tree.into())?;
        }

        Ok(builder.write()?)
    }

    fn remove_from_path(
        &self,
        tree: &git2::Tree<'repo>,
        components: &[PathBuf],
        depth: usize,
    ) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(Some(tree))?;

        if depth >= components.len() - 1 {
            // We're at the parent directory of the target - remove the entry
            let target = &components[depth];
            // Check if entry exists before removing
            match tree.get_path(target) {
                Ok(_) => builder.remove(target)?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    // Entry doesn't exist - tree unchanged
                    return Ok(tree.id());
                }
                Err(e) => return Err(Error::Git(e)),
            }
        } else {
            // We need to go deeper
            let component = &components[depth];
            let subtree = match tree.get_path(component) {
                Ok(entry) => self.repo.find_tree(entry.id())?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    // Path doesn't exist - nothing to remove
                    return Ok(tree.id());
                }
                Err(e) => return Err(Error::Git(e)),
            };

            // Recursively update the subtree
            let new_subtree_oid = self.remove_from_path(&subtree, components, depth + 1)?;

            // Update the entry in the current tree
            builder.insert(component, new_subtree_oid, git2::FileMode::Tree.into())?;
        }

        Ok(builder.write()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn insert_file_in_root() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("test.txt", "hello")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        let ext = TreeBuilderExt::new(&repo.repo);
        let blob_oid = repo.repo.blob(b"modified")?;
        let new_tree_oid = ext.insert_file(&tree, Path::new("test.txt"), blob_oid, 0o100644)?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;
        assert_eq!(new_tree.len(), 1);
        let entry = new_tree.get_path(Path::new("test.txt"))?;
        assert_eq!(entry.id(), blob_oid);

        Ok(())
    }

    #[test]
    fn insert_file_in_nested_directory() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("src/main.rs", "fn main() {}")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        let ext = TreeBuilderExt::new(&repo.repo);
        let blob_oid = repo.repo.blob(b"fn updated() {}")?;
        let new_tree_oid = ext.insert_file(&tree, Path::new("src/main.rs"), blob_oid, 0o100644)?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;
        let entry = new_tree.get_path(Path::new("src/main.rs"))?;
        assert_eq!(entry.id(), blob_oid);

        Ok(())
    }

    #[test]
    fn insert_file_creates_intermediate_directories() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("root.txt", "root")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        let ext = TreeBuilderExt::new(&repo.repo);
        let blob_oid = repo.repo.blob(b"deep content")?;
        let new_tree_oid = ext.insert_file(
            &tree,
            Path::new("deeply/nested/path/file.rs"),
            blob_oid,
            0o100644,
        )?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;

        // Verify the file exists at the deep path
        let entry = new_tree.get_path(Path::new("deeply/nested/path/file.rs"))?;
        assert_eq!(entry.id(), blob_oid);

        // Verify original file still exists
        let root_entry = new_tree.get_path(Path::new("root.txt"))?;
        assert!(root_entry.id() != blob_oid);

        Ok(())
    }

    #[test]
    fn remove_file_in_root() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("keep.txt", "keep")?;
        repo.write_file("remove.txt", "remove")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;
        assert_eq!(tree.len(), 2);

        let ext = TreeBuilderExt::new(&repo.repo);
        let new_tree_oid = ext.remove_path(&tree, Path::new("remove.txt"))?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;
        assert_eq!(new_tree.len(), 1);
        assert!(new_tree.get_path(Path::new("keep.txt")).is_ok());
        assert!(new_tree.get_path(Path::new("remove.txt")).is_err());

        Ok(())
    }

    #[test]
    fn remove_file_in_nested_directory() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("src/main.rs", "fn main() {}")?;
        repo.write_file("src/lib.rs", "pub fn lib() {}")?;
        repo.write_file("other.txt", "other")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        let ext = TreeBuilderExt::new(&repo.repo);
        let new_tree_oid = ext.remove_path(&tree, Path::new("src/main.rs"))?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;

        // main.rs should be removed
        assert!(new_tree.get_path(Path::new("src/main.rs")).is_err());

        // lib.rs should still exist
        assert!(new_tree.get_path(Path::new("src/lib.rs")).is_ok());

        // other.txt should still exist
        assert!(new_tree.get_path(Path::new("other.txt")).is_ok());

        Ok(())
    }

    #[test]
    fn remove_nonexistent_path_returns_unchanged_tree() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("existing.txt", "exists")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;
        let original_oid = tree.id();

        let ext = TreeBuilderExt::new(&repo.repo);
        let new_tree_oid = ext.remove_path(&tree, Path::new("nonexistent.txt"))?;

        // Since path doesn't exist, tree should be unchanged
        assert_eq!(new_tree_oid, original_oid);

        Ok(())
    }

    #[test]
    fn update_existing_nested_file() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("src/deep/file.rs", "original")?;
        repo.write_file("src/other.rs", "other")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        let ext = TreeBuilderExt::new(&repo.repo);
        let new_blob_oid = repo.repo.blob(b"updated content")?;
        let new_tree_oid =
            ext.insert_file(&tree, Path::new("src/deep/file.rs"), new_blob_oid, 0o100644)?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;

        // File should be updated
        let entry = new_tree.get_path(Path::new("src/deep/file.rs"))?;
        assert_eq!(entry.id(), new_blob_oid);

        // Other file should be unchanged
        let other_entry = new_tree.get_path(Path::new("src/other.rs"))?;
        let original_other_blob = repo.repo.blob(b"other")?;
        assert_eq!(other_entry.id(), original_other_blob);

        Ok(())
    }

    #[test]
    fn deeply_nested_path_operations() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("a/b/c/d/e/f/file.txt", "deep")?;
        let commit = repo.commit("initial")?.created;
        let tree = repo.repo.find_commit(commit.oid())?.tree()?;

        // Test insert at deep level
        let ext = TreeBuilderExt::new(&repo.repo);
        let new_blob = repo.repo.blob(b"new deep")?;
        let new_tree_oid = ext.insert_file(
            &tree,
            Path::new("a/b/c/d/e/f/another.txt"),
            new_blob,
            0o100644,
        )?;

        let new_tree = repo.repo.find_tree(new_tree_oid)?;
        assert!(new_tree.get_path(Path::new("a/b/c/d/e/f/file.txt")).is_ok());
        assert!(
            new_tree
                .get_path(Path::new("a/b/c/d/e/f/another.txt"))
                .is_ok()
        );

        // Test remove at deep level
        let final_tree_oid = ext.remove_path(&new_tree, Path::new("a/b/c/d/e/f/file.txt"))?;
        let final_tree = repo.repo.find_tree(final_tree_oid)?;
        assert!(
            final_tree
                .get_path(Path::new("a/b/c/d/e/f/file.txt"))
                .is_err()
        );
        assert!(
            final_tree
                .get_path(Path::new("a/b/c/d/e/f/another.txt"))
                .is_ok()
        );

        Ok(())
    }
}
