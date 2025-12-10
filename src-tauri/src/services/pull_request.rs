use git2::Oid;

use crate::db::DB;
use crate::errors::{CommandError, Result};
use crate::models::{GetPullResponse, PRCommit, PullRequest};
use crate::services::GitHubService;

pub struct PullRequestService;

impl PullRequestService {
    pub async fn list_pull_requests(
        github: &GitHubService,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>> {
        let prs = github.list_pull_requests(owner, repo).await?;
        Ok(prs.into_iter().map(PullRequest::from).collect())
    }

    pub async fn get_pull_request_details(
        github: &GitHubService,
        db: &mut DB,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<GetPullResponse> {
        let pr = github.get_pull_request(owner, repo, pr_number).await?;
        let gh_repo = github.get_repository(owner, repo).await?;

        let repo_node_id = gh_repo.node_id.ok_or_else(|| {
            log::error!("Got null node id");
            CommandError::Internal
        })?;

        let repo_dir = db
            .find_local_repo(&repo_node_id)
            .await
            .map_err(|err| {
                log::error!("DB error: {err}");
                CommandError::Internal
            })?
            .ok_or_else(|| CommandError::bad_input("Please set local repository to review PR"))?;

        let repository = git2::Repository::open(&repo_dir.local_dir).map_err(|err| {
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

            let change_id = commit
                .header_field_bytes("change-id")
                .ok()
                .and_then(|buf| buf.as_str().map(String::from));

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
        })
    }
}
