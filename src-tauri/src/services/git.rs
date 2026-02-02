use git2::{Commit, Oid, Repository};

use crate::models::{ChangeId, PRCommit};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Invalid SHA: {0}")]
    InvalidSha(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
}

pub struct GitService;

impl GitService {
    pub fn open_repository(local_dir: &str) -> Result<Repository> {
        Repository::open(local_dir).map_err(|_| Error::RepoNotFound(local_dir.to_string()))
    }

    pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
        commit
            .header_field_bytes("change-id")
            .ok()
            .and_then(|buf| buf.as_str().map(String::from).map(ChangeId::from))
    }

    pub fn get_commits_in_range(
        repo: &Repository,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<Vec<PRCommit>> {
        let head_oid =
            Oid::from_str(head_sha).map_err(|_| Error::InvalidSha(head_sha.to_string()))?;

        let base_oid =
            Oid::from_str(base_sha).map_err(|_| Error::InvalidSha(base_sha.to_string()))?;

        let mut walker = repo.revwalk()?;

        let range = format!("{}..{}", base_oid, head_oid);
        walker.push_range(&range)?;

        let mut commits = Vec::new();
        for oid in walker {
            let oid = oid?;
            let commit = repo
                .find_commit(oid)
                .map_err(|_| Error::CommitNotFound(oid.to_string()))?;

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
