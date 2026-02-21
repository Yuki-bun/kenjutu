mod file_diff;
mod file_list;

use super::git;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File not found in tree: {0}")]
    FileNotFound(String),

    #[error("Git error: {0}")]
    Git(#[from] git::Error),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Marker commit error: {0}")]
    MarkerCommit(#[from] marker_commit::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Conflicted parents in merge commit: {0}")]
    MergeConflict(git2::Oid),
}

pub use file_diff::{generate_single_file_diff, get_context_lines};
pub use file_list::generate_file_list;

/// For merge commits with exactly 2 parents, compute the auto-merged tree
/// via `merge_trees()`. Returns `Some(tree)` to use as the diff base instead
/// of parent(0)'s tree. Returns `None` for non-merge commits, octopus merges,
/// or when the auto-merge has conflicts (falls back to parent(0) diff).
fn compute_merge_base_tree<'repo>(
    repo: &'repo git2::Repository,
    commit: &git2::Commit,
) -> Result<Option<git2::Tree<'repo>>> {
    if commit.parent_count() != 2 {
        return Ok(None);
    }

    let parent0 = commit.parent(0)?;
    let parent1 = commit.parent(1)?;

    let ancestor_oid = match repo.merge_base(parent0.id(), parent1.id()) {
        Ok(oid) => oid,
        Err(_) => return Ok(None),
    };
    let ancestor = repo.find_commit(ancestor_oid)?;

    let mut index =
        repo.merge_trees(&ancestor.tree()?, &parent0.tree()?, &parent1.tree()?, None)?;

    if index.has_conflicts() {
        return Ok(None);
    }

    let tree_oid = index.write_tree_to(repo)?;
    Ok(Some(repo.find_tree(tree_oid)?))
}
