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
    /// Removing a non-existent path will not error and will be a
    /// no-op (returning the original tree OID).
    #[allow(dead_code)]
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
            builder.insert(filename, blob_oid, filemode)?;
        } else {
            let component = &components[depth];
            let subtree = match tree.get_path(component) {
                Ok(entry) => self.repo.find_tree(entry.id())?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    let empty_oid = self.repo.treebuilder(None)?.write()?;
                    self.repo.find_tree(empty_oid)?
                }
                Err(e) => return Err(Error::Git(e)),
            };

            let new_subtree_oid = self.upsert_path(
                &subtree,
                components,
                depth + 1,
                filename,
                blob_oid,
                filemode,
            )?;

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
            let target = &components[depth];
            match tree.get_path(target) {
                Ok(_) => builder.remove(target)?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    return Ok(tree.id());
                }
                Err(e) => return Err(Error::Git(e)),
            }
        } else {
            let component = &components[depth];
            let subtree = match tree.get_path(component) {
                Ok(entry) => self.repo.find_tree(entry.id())?,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    return Ok(tree.id());
                }
                Err(e) => return Err(Error::Git(e)),
            };

            let new_subtree_oid = self.remove_from_path(&subtree, components, depth + 1)?;
            builder.insert(component, new_subtree_oid, git2::FileMode::Tree.into())?;
        }

        Ok(builder.write()?)
    }
}
