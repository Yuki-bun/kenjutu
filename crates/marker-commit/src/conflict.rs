use std::path::Path;

use crate::Result;
use git2::{Index, Oid, Repository};

pub fn resolve_conflict_prefer_our(
    repo: &Repository,
    index: &mut Index,
) -> Result<Oid, git2::Error> {
    let mut resolutions = Vec::new();
    let conflicts = index.conflicts()?;

    for conflict in conflicts {
        let c = conflict?;

        let path_bytes = c
            .ancestor
            .as_ref()
            .or(c.our.as_ref())
            .or(c.their.as_ref())
            .map(|e| e.path.clone())
            .expect("conflict entry must have at least one stage with a path");

        resolutions.push((path_bytes, c.our));
    }

    for (path_vec, our_entry) in resolutions {
        let path = Path::new(std::str::from_utf8(&path_vec).unwrap());

        index.conflict_remove(path)?;

        if let Some(mut entry) = our_entry {
            entry.flags = 0;
            index.add(&entry)?;
        } else {
            // 'Ours' was a deletion, so ensure it's gone from all stages
            index.remove_path(path)?;
        }
    }

    let tree_oid = index.write_tree_to(repo)?;
    Ok(tree_oid)
}

#[cfg(test)]
mod tests {
    use test_repo::TestRepo;

    use super::*;

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_resolve_conflict_prefer_ours() -> Result {
        // B= R      B'  R'
        // | /   ->  | /
        // A         A'
        let repo = TestRepo::new().unwrap();
        repo.write_file("file.txt", "A")?;
        let a = repo.commit("a")?.created;
        let a_tree = repo.repo.find_commit(a.oid())?.tree()?;
        repo.write_file("file.txt", "B")?;
        repo.commit("b")?;
        repo.new_revision(a.change_id)?;
        repo.write_file("file.txt", "B")?;
        let r = repo.commit("r")?.created;
        let r_tree = repo.repo.find_commit(r.oid())?.tree()?;

        repo.edit(a.change_id)?;
        repo.write_file("file.txt", "A'")?;
        let a_prime = repo.commit("a'")?.created;
        let a_prime_tree = repo.repo.find_commit(a_prime.oid())?.tree()?;
        repo.edit(a.change_id)?;
        repo.write_file("file.txt", "B'")?;
        repo.commit("b'")?;

        let mut r_prime = repo
            .repo
            .merge_trees(&a_tree, &a_prime_tree, &r_tree, None)?;

        assert!(r_prime.has_conflicts());

        let resolved_oid = resolve_conflict_prefer_our(&repo.repo, &mut r_prime)?;
        let resolved_tree = repo.repo.find_tree(resolved_oid)?;

        // The content should match "A'"
        let entry = resolved_tree.get_name("file.txt").unwrap();
        let blob = repo.repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content()).unwrap();
        assert_eq!(content, "A'");

        Ok(())
    }
}
