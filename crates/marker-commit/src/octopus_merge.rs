use crate::{Error, Result};
use git2::{Commit, Oid, Repository};

/// Performs a octopus merge of the commit trees.
/// Returns Ok(Some(tree_oid)) if successful, Ok(None) if there are conflicts, or Err on error.
pub(crate) fn octopus_merge(repo: &Repository, commits: &[Commit]) -> Result<Option<Oid>> {
    if commits.is_empty() {
        return Err(Error::Internal(
            "No commits provided for mega-merge".to_string(),
        ));
    }
    if commits.len() == 1 {
        return Ok(Some(commits[0].tree_id()));
    }

    let oids: Vec<Oid> = commits.iter().map(|c| c.id()).collect();
    let ancestor_oid = repo.merge_base_many(&oids)?;
    let ancestor_tree = repo.find_commit(ancestor_oid)?.tree()?;

    let mut current_tree = commits[0].tree()?;

    for commit in commits[1..].iter() {
        let mut index = repo.merge_trees(&ancestor_tree, &current_tree, &commit.tree()?, None)?;

        if index.has_conflicts() {
            return Ok(None);
        }

        let next_oid = index.write_tree_to(repo)?;
        current_tree = repo.find_tree(next_oid)?;
    }

    Ok(Some(current_tree.id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn empty_commits_returns_error() -> Result {
        let repo = TestRepo::new()?;
        assert!(octopus_merge(&repo.repo, &[]).is_err());
        Ok(())
    }

    #[test]
    fn single_commit_returns_its_tree() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("file", "content")?;
        let a = repo.commit("A")?.created;
        let commit = repo.repo.find_commit(a.oid())?;
        let tree_id = commit.tree_id();
        let result = octopus_merge(&repo.repo, &[commit])?;
        assert_eq!(result, Some(tree_id));
        Ok(())
    }

    #[test]
    fn merges_two_non_conflicting_branches() -> Result {
        // A (base) -- B (adds file_b)
        //          \- C (adds file_c)
        let repo = TestRepo::new()?;
        repo.write_file("base", "base")?;
        let a = repo.commit("A")?.created;

        repo.write_file("file_b", "b content")?;
        let b = repo.commit("B")?.created;

        repo.merge(&[a.change_id], "C")?;
        repo.write_file("file_c", "c content")?;
        let c = repo.work_copy()?;

        let b_commit = repo.repo.find_commit(b.oid())?;
        let c_commit = repo.repo.find_commit(c.oid())?;
        let result = octopus_merge(&repo.repo, &[b_commit, c_commit])?;

        assert!(result.is_some());
        let merged_tree = repo.repo.find_tree(result.unwrap())?;
        assert!(
            merged_tree.get_name("file_b").is_some(),
            "file_b missing from merged tree"
        );
        assert!(
            merged_tree.get_name("file_c").is_some(),
            "file_c missing from merged tree"
        );
        Ok(())
    }

    #[test]
    fn returns_none_for_conflicting_branches() -> Result {
        // A (file1="base") -- B (file1="from B")
        //                  \- C (file1="from C")
        let repo = TestRepo::new()?;
        repo.write_file("file1", "base")?;
        let a = repo.commit("A")?.created;

        repo.write_file("file1", "from B")?;
        let b = repo.commit("B")?.created;

        repo.merge(&[a.change_id], "C")?;
        repo.write_file("file1", "from C")?;
        let c = repo.work_copy()?;

        let b_commit = repo.repo.find_commit(b.oid())?;
        let c_commit = repo.repo.find_commit(c.oid())?;
        let result = octopus_merge(&repo.repo, &[b_commit, c_commit])?;

        assert_eq!(result, None, "conflicting branches should return None");
        Ok(())
    }

    #[test]
    fn merges_three_non_conflicting_branches() -> Result {
        // A (base) -- B (adds file_b)
        //          \- C (adds file_c)
        //          \- D (adds file_d)
        let repo = TestRepo::new()?;
        repo.write_file("base", "base")?;
        let a = repo.commit("A")?.created;

        repo.write_file("file_b", "b")?;
        let b = repo.commit("B")?.created;

        repo.merge(&[a.change_id], "C")?;
        repo.write_file("file_c", "c")?;
        let c = repo.work_copy()?;

        repo.merge(&[a.change_id], "D")?;
        repo.write_file("file_d", "d")?;
        let d = repo.work_copy()?;

        let b_commit = repo.repo.find_commit(b.oid())?;
        let c_commit = repo.repo.find_commit(c.oid())?;
        let d_commit = repo.repo.find_commit(d.oid())?;
        let result = octopus_merge(&repo.repo, &[b_commit, c_commit, d_commit])?;

        assert!(result.is_some());
        let merged_tree = repo.repo.find_tree(result.unwrap())?;
        assert!(merged_tree.get_name("file_b").is_some(), "file_b missing");
        assert!(merged_tree.get_name("file_c").is_some(), "file_c missing");
        assert!(merged_tree.get_name("file_d").is_some(), "file_d missing");
        Ok(())
    }
}
