use crate::{Error, Result};
use git2::{Commit, MergeFileOptions, Oid, Repository, Tree};
use std::path::Path;

/// Returns the effective tree for a commit.
///
/// For jj conflicted commits (those with a `jj:trees` header), materializes the
/// conflict into a single tree with conflict markers. For normal commits, returns
/// `commit.tree()`.
pub fn materialize_tree<'a>(repo: &'a Repository, commit: &Commit<'a>) -> Result<Tree<'a>> {
    let header_bytes = match commit.header_field_bytes("jj:trees") {
        Ok(bytes) => bytes,
        Err(_) => return Ok(commit.tree()?),
    };

    let header_str = std::str::from_utf8(&header_bytes).map_err(|e| {
        Error::Internal(format!(
            "Failed to parse jj:trees header for commit {}: {}",
            commit.id(),
            e
        ))
    })?;

    let oids: Vec<Oid> = header_str
        .split_whitespace()
        .map(|hex| {
            Oid::from_str(hex).map_err(|e| {
                Error::Internal(format!(
                    "Failed to parse jj:trees OID '{}' for commit {}: {}",
                    hex,
                    commit.id(),
                    e
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;

    if oids.len() < 3 || oids.len().is_multiple_of(2) {
        return Err(Error::Internal(format!(
            "Failed to parse jj:trees header for commit {}: expected odd count >= 3, got {}",
            commit.id(),
            oids.len()
        )));
    }

    let trees: Vec<Tree<'a>> = oids
        .iter()
        .map(|oid| repo.find_tree(*oid).map_err(Error::Git))
        .collect::<Result<Vec<_>>>()?;

    let mut result_tree = merge_three(repo, &trees[1], &trees[0], &trees[2])?;

    for pair in trees[3..].chunks(2) {
        result_tree = merge_three(repo, &pair[0], &result_tree, &pair[1])?;
    }

    Ok(result_tree)
}

fn merge_three<'a>(
    repo: &'a Repository,
    base: &Tree<'a>,
    ours: &Tree<'a>,
    theirs: &Tree<'a>,
) -> Result<Tree<'a>> {
    let mut index = repo.merge_trees(base, ours, theirs, None)?;

    if index.has_conflicts() {
        write_conflict_markers(repo, &mut index)?;
    }

    let tree_oid = index.write_tree_to(repo)?;
    Ok(repo.find_tree(tree_oid)?)
}

/// Replace conflicted index entries with blobs containing conflict markers.
///
/// Uses `merge_file_from_index` so only the conflicting regions get markers
/// while the rest of the file content is preserved.
pub(crate) fn write_conflict_markers(repo: &Repository, index: &mut git2::Index) -> Result<()> {
    let mut resolutions: Vec<(Vec<u8>, Oid, u32)> = Vec::new();

    let mut opts = MergeFileOptions::new();
    opts.our_label("Side 1").their_label("Side 2");

    for conflict in index.conflicts()? {
        let c = conflict?;

        let ancestor = c.ancestor.as_ref();
        let ours = c.our.as_ref();
        let theirs = c.their.as_ref();

        let path_bytes = ancestor
            .or(ours)
            .or(theirs)
            .map(|e| e.path.clone())
            .expect("conflict entry must have at least one stage with a path");

        let merged = match (ancestor, ours, theirs) {
            (Some(a), Some(o), Some(t)) => repo.merge_file_from_index(a, o, t, Some(&mut opts))?,
            // Add/add conflict — both sides added the file with no common ancestor.
            // Synthesize an empty ancestor so merge_file_from_index produces conflict markers.
            (None, Some(o), Some(t)) => {
                let empty_blob = repo.blob(&[])?;
                let synthetic_ancestor = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: o.mode,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: empty_blob,
                    flags: 0,
                    flags_extended: 0,
                    path: path_bytes.clone(),
                };
                repo.merge_file_from_index(&synthetic_ancestor, o, t, Some(&mut opts))?
            }
            // One side deleted — take the surviving side's content
            (_, Some(surviving), None) | (_, None, Some(surviving)) => {
                resolutions.push((path_bytes, surviving.id, surviving.mode));
                continue;
            }
            _ => continue,
        };

        let blob_oid = repo.blob(merged.content())?;
        let mode = merged.mode();

        resolutions.push((path_bytes, blob_oid, mode));
    }

    for (path_bytes, blob_oid, mode) in resolutions {
        let path = Path::new(
            std::str::from_utf8(&path_bytes)
                .map_err(|e| Error::Internal(format!("non-UTF-8 conflict path: {}", e)))?,
        );
        index.conflict_remove(path)?;

        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: blob_oid,
            flags: 0,
            flags_extended: 0,
            path: path_bytes,
        };
        index.add(&entry)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn normal_commit_returns_commit_tree() -> Result {
        let repo = TestRepo::new()?;
        repo.write_file("file.txt", "hello")?;
        let a = repo.commit("A")?.created;
        let commit = repo.repo.find_commit(a.oid())?;

        let tree = materialize_tree(&repo.repo, &commit)?;
        assert_eq!(tree.id(), commit.tree()?.id());
        Ok(())
    }

    #[test]
    fn conflicted_jj_commit_returns_merged_tree() -> Result {
        // Create a real jj conflict:
        //   base: file.txt = "base\n"
        //   side1: file.txt = "side1\n"
        //   side2: file.txt = "side2\n"
        //   merge side1 + side2 -> jj creates a conflicted commit with jj:trees header
        let repo = TestRepo::new()?;
        repo.write_file("file.txt", "base\n")?;
        let base = repo.commit("base")?.created;

        // side1
        repo.write_file("file.txt", "side1\n")?;
        let side1 = repo.commit("side1")?.created;

        // side2: branch off base
        repo.new_revision(base.change_id)?;
        repo.write_file("file.txt", "side2\n")?;
        let side2 = repo.commit("side2")?.created;

        // merge creates a conflicted commit
        let merge = repo.merge(&[side1.change_id, side2.change_id], "merge")?;
        // The working copy after `jj new side1 side2` is the conflicted commit
        let merge_commit = repo.repo.find_commit(merge.oid())?;

        let tree = materialize_tree(&repo.repo, &merge_commit)?;

        // The materialized tree should have file.txt (not .jjconflict-* directories)
        let entry = tree.get_name("file.txt");
        assert!(
            entry.is_some(),
            "materialized tree should have file.txt, not synthetic conflict directories"
        );

        let blob = repo.repo.find_blob(entry.unwrap().id())?;
        let content = std::str::from_utf8(blob.content())?;
        assert_eq!(
            content,
            "<<<<<<< Side 1\nside1\n=======\nside2\n>>>>>>> Side 2\n"
        );

        Ok(())
    }

    #[test]
    fn conflicted_jj_commit_non_conflicting_files_preserved() -> Result {
        // base: a.txt = "a", b.txt = "b"
        // side1: a.txt = "a1"  (b.txt unchanged)
        // side2: a.txt = "a2"  (b.txt unchanged)
        // merge -> conflict on a.txt, b.txt should be fine
        let repo = TestRepo::new()?;
        repo.write_file("a.txt", "a\n")?;
        repo.write_file("b.txt", "b\n")?;
        let base = repo.commit("base")?.created;

        repo.write_file("a.txt", "a1\n")?;
        let side1 = repo.commit("side1")?.created;

        repo.new_revision(base.change_id)?;
        repo.write_file("a.txt", "a2\n")?;
        let side2 = repo.commit("side2")?.created;

        let merge = repo.merge(&[side1.change_id, side2.change_id], "merge")?;
        let merge_commit = repo.repo.find_commit(merge.oid())?;

        let tree = materialize_tree(&repo.repo, &merge_commit)?;

        // b.txt should be preserved unchanged
        let b_entry = tree
            .get_name("b.txt")
            .expect("b.txt should exist in materialized tree");
        let b_blob = repo.repo.find_blob(b_entry.id())?;
        assert_eq!(std::str::from_utf8(b_blob.content())?, "b\n");

        Ok(())
    }

    #[test]
    fn both_add_conflict() -> Result {
        // base: (empty)
        // side1: a.txt = "a1"
        // side2: a.txt = "a2"
        // merge -> both add a.txt with no common ancestor, should still get conflict markers
        let repo = TestRepo::new()?;
        let base = repo.commit("base")?.created;

        repo.write_file("a.txt", "a1\n")?;
        repo.commit("side1 change")?;
        let side1 = repo.commit("side1")?.created;

        repo.new_revision(base.change_id)?;
        repo.write_file("a.txt", "a2\n")?;
        let side2 = repo.commit("side2 change")?.created;

        let merge = repo.merge(&[side1.change_id, side2.change_id], "merge")?;
        let merge_commit = repo.repo.find_commit(merge.oid())?;

        let tree = materialize_tree(&repo.repo, &merge_commit)?;

        let entry = tree
            .get_name("a.txt")
            .expect("a.txt should exist in materialized tree");
        let blob = repo.repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content())?;
        assert_eq!(content, "<<<<<<< Side 1\na1\n=======\na2\n>>>>>>> Side 2\n");

        Ok(())
    }
}
