use std::convert::identity;

use git2::Oid;

use crate::db::DB;
use crate::errors::{CommandError, Result};
use crate::models::{GetPullResponse, MergePullResponse, PRCommit, PullRequest};
use crate::services::{GitHubService, GitService, RepositoryCacheService};

pub struct PullRequestService;

impl PullRequestService {
    pub async fn list_pull_requests(
        github: &GitHubService,
        db: &mut DB,
        node_id: &str,
    ) -> Result<Vec<PullRequest>> {
        // Get owner/name from cache
        let (owner, repo) =
            RepositoryCacheService::get_repo_owner_name(github, db, node_id).await?;

        let prs = github.list_pull_requests(&owner, &repo).await?;
        Ok(prs.into_iter().map(PullRequest::from).collect())
    }

    pub async fn get_pull_request_details(
        github: &GitHubService,
        db: &mut DB,
        node_id: &str,
        pr_number: u64,
    ) -> Result<GetPullResponse> {
        // Get owner/name from cache
        let (owner, repo) =
            RepositoryCacheService::get_repo_owner_name(github, db, node_id).await?;

        let pr = github.get_pull_request(&owner, &repo, pr_number).await?;

        let repo_dir = db
            .find_local_repo(node_id)
            .map_err(|err| {
                log::error!("DB error: {err}");
                CommandError::Internal
            })?
            .ok_or_else(|| CommandError::bad_input("Please set local repository to review PR"))?;

        let local_dir = repo_dir
            .local_dir
            .ok_or_else(|| CommandError::bad_input("Please set local repository to review PR"))?;

        let repository = git2::Repository::open(&local_dir).map_err(|err| {
            log::error!("Could not find local repository: {err}");
            CommandError::bad_input(
                "Could not connect to repository set by user. Please reset local repository for this repository",
            )
        })?;

        let head_sha = Oid::from_str(&pr.head.sha).map_err(|err| {
            log::error!("GitHub gave me a wrong hash: {err}");
            CommandError::Internal
        })?;
        let base_sha = Oid::from_str(&pr.base.sha).map_err(|err| {
            log::error!("GitHub gave me a wrong hash: {err}");
            CommandError::Internal
        })?;

        let mut walker = repository.revwalk().map_err(|err| {
            log::error!("Failed to initiate rev walker: {err}");
            CommandError::Internal
        })?;
        let range = format!("{}..{}", base_sha, head_sha);
        walker.push_range(&range).map_err(|err| {
            log::error!("Failed to push range to walker: {err}");
            CommandError::Internal
        })?;

        let mut commits: Vec<PRCommit> = Vec::new();
        for oid in walker {
            let oid = oid.map_err(|err| {
                log::error!("Walker error: {err}");
                CommandError::Internal
            })?;
            let commit = repository.find_commit(oid).map_err(|err| {
                log::error!("Could not find commit: {err}");
                CommandError::Internal
            })?;

            let change_id = GitService::get_change_id(&commit);

            let commit = PRCommit {
                change_id,
                sha: oid.to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                description: commit.body().unwrap_or("").to_string(),
            };
            commits.push(commit);
        }

        Ok(GetPullResponse {
            title: pr.title.unwrap_or_default(),
            body: pr.body.unwrap_or_default(),
            base_branch: pr.base.ref_field,
            head_branch: pr.head.ref_field,
            commits,
            mergable: pr.mergeable.is_some_and(identity),
        })
    }

    pub async fn merge_pull_request(
        github: &GitHubService,
        db: &mut DB,
        node_id: &str,
        pr_number: u64,
    ) -> Result<MergePullResponse> {
        // Get owner/name from cache
        let (owner, repo) =
            RepositoryCacheService::get_repo_owner_name(github, db, node_id).await?;

        // Call GitHub API to merge
        let merge_result = github.merge_pull_request(&owner, &repo, pr_number).await?;

        Ok(MergePullResponse {
            sha: merge_result.sha.unwrap_or_default(),
            merged: merge_result.merged,
            message: merge_result.message.unwrap_or_else(|| {
                "Pull request merged successfully".to_string()
            }),
        })
    }
}
