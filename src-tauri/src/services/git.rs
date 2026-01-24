use git2::{Commit, Oid, Repository};

use crate::errors::{CommandError, Result};
use crate::models::{ChangeId, PRCommit};

pub struct GitService;

impl GitService {
    pub fn open_repository(local_dir: &str) -> Result<Repository> {
        Repository::open(local_dir).map_err(|err| {
            log::error!("Could not open repository: {err}");
            CommandError::bad_input("Failed to open repository")
        })
    }

    pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
        commit
            .header_field_bytes("change-id")
            .ok()
            .and_then(|buf| buf.as_str().map(String::from).map(ChangeId::from))
    }

    pub fn get_blob_content(repo: &Repository, oid: Oid) -> Option<String> {
        if oid.is_zero() {
            return None;
        }
        let blob = repo.find_blob(oid).ok()?;
        if blob.is_binary() {
            return None;
        }
        std::str::from_utf8(blob.content())
            .ok()
            .map(|s| s.to_string())
    }

    pub fn get_commits_in_range(
        repo: &Repository,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<Vec<PRCommit>> {
        let head_oid = Oid::from_str(head_sha).map_err(|err| {
            log::error!("Invalid head SHA: {err}");
            CommandError::bad_input("Invalid head SHA")
        })?;

        let base_oid = Oid::from_str(base_sha).map_err(|err| {
            log::error!("Invalid base SHA: {err}");
            CommandError::bad_input("Invalid base SHA")
        })?;

        let mut walker = repo.revwalk().map_err(|err| {
            log::error!("Failed to initiate rev walker: {err}");
            CommandError::Internal
        })?;

        let range = format!("{}..{}", base_oid, head_oid);
        walker.push_range(&range).map_err(|err| {
            log::error!("Failed to push range to walker: {err}");
            CommandError::Internal
        })?;

        let mut commits = Vec::new();
        for oid in walker {
            let oid = oid.map_err(|err| {
                log::error!("Walker error: {err}");
                CommandError::Internal
            })?;
            let commit = repo.find_commit(oid).map_err(|err| {
                log::error!("Could not find commit: {err}");
                CommandError::Internal
            })?;

            let change_id = Self::get_change_id(&commit);

            let pr_commit = PRCommit {
                change_id,
                sha: oid.to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                description: commit.body().unwrap_or("").to_string(),
            };
            commits.push(pr_commit);
        }

        Ok(commits)
    }
}
