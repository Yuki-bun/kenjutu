use git2::{Commit, Oid, Repository};
use octocrab::models::pulls::PullRequest;

enum Error {
    Git(git2::Error),
}

impl From<git2::Error> for Error {
    fn from(value: git2::Error) -> Self {
        Self::Git(value)
    }
}

pub struct PRAnalyzer<'repo> {
    base: Commit<'repo>,
    head: Commit<'repo>,
}

impl<'repo> PRAnalyzer<'repo> {
    fn from_gh_pr(pr: PullRequest, repo: &'repo Repository) -> Result<Self, Error> {
        let base_oid = Oid::from_str(&pr.base.sha)?;
        let base = repo.find_commit(base_oid)?;
        let head_oid = Oid::from_str(&pr.head.sha)?;
        let head = repo.find_commit(head_oid)?;
        Ok(Self { base, head })
    }
}
