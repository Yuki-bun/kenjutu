use crate::{
    Error, Result,
    materialize_tree::{materialize_tree, write_conflict_markers},
};
use git2::{Commit, Oid, Repository};

/// Performs an octopus merge of the commit trees.
/// Conflicts are written with conflict markers rather than returning an error.
pub(crate) fn octopus_merge(repo: &Repository, commits: &[Commit]) -> Result<Oid> {
    if commits.is_empty() {
        return Err(Error::Internal(
            "No commits provided for mega-merge".to_string(),
        ));
    }
    if commits.len() == 1 {
        return Ok(materialize_tree(repo, &commits[0])?.id());
    }

    let oids: Vec<Oid> = commits.iter().map(|c| c.id()).collect();
    let ancestor_oid = repo.merge_base_many(&oids)?;
    let ancestor_tree = repo.find_commit(ancestor_oid)?.tree()?;

    let mut current_tree = materialize_tree(repo, &commits[0])?;

    for commit in commits[1..].iter() {
        let mut index = repo.merge_trees(
            &ancestor_tree,
            &current_tree,
            &materialize_tree(repo, commit)?,
            None,
        )?;

        if index.has_conflicts() {
            write_conflict_markers(repo, &mut index)?;
        }

        let next_oid = index.write_tree_to(repo)?;
        current_tree = repo.find_tree(next_oid)?;
    }

    Ok(current_tree.id())
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
        assert_eq!(result, tree_id);
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

        let merged_tree = repo.repo.find_tree(result)?;
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
    fn conflicting_branches_produce_conflict_markers() -> Result {
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

        let merged_tree = repo.repo.find_tree(result)?;
        let entry = merged_tree.get_name("file1").expect("file1 should exist");
        let blob = repo.repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content())?;
        assert_eq!(
            content,
            "<<<<<<< Side 1\nfrom B\n=======\nfrom C\n>>>>>>> Side 2\n"
        );
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

        let merged_tree = repo.repo.find_tree(result)?;
        assert!(merged_tree.get_name("file_b").is_some(), "file_b missing");
        assert!(merged_tree.get_name("file_c").is_some(), "file_c missing");
        assert!(merged_tree.get_name("file_d").is_some(), "file_d missing");
        Ok(())
    }
}
