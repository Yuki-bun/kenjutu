use std::path::PathBuf;

use crate::db::{LocalRepo, DB};
use crate::errors::{CommandError, Result};
use crate::models::{FullRepo, GhRepoId, Repo};
use crate::services::{GitHubService, RepositoryCacheService};

pub struct RepositoryService;

impl RepositoryService {
    pub async fn get_repositories(github: &GitHubService) -> Result<Vec<Repo>> {
        let repos = github.list_repositories().await?;
        Ok(repos.into_iter().map(Repo::from).collect())
    }

    pub async fn get_repository(
        github: &GitHubService,
        db: &mut DB,
        repo_id: &GhRepoId,
    ) -> Result<FullRepo> {
        // Get owner/name from cache
        let (owner, name) =
            RepositoryCacheService::get_repo_owner_name(github, db, repo_id).await?;

        // Fetch fresh data from GitHub
        let repo = github.get_repository(&owner, &name).await?;

        let local_dir = db.find_repository(repo_id).map_or_else(
            |err| {
                log::error!("DB error: {err}");
                None
            },
            |repo| repo.and_then(|r| r.local_dir.map(PathBuf::from)),
        );

        Ok(FullRepo::new(repo, local_dir))
    }

    pub async fn set_local_repository(
        github: &GitHubService,
        db: &mut DB,
        repo_id: &GhRepoId,
        local_dir: &str,
    ) -> Result<()> {
        // Validate git repository
        if git2::Repository::open(local_dir).is_err() {
            return Err(CommandError::bad_input(format!(
                "Directory {} is not a git repository",
                local_dir
            )));
        }

        // Get owner/name from cache (will fetch if needed)
        let (owner, name) =
            RepositoryCacheService::get_repo_owner_name(github, db, repo_id).await?;

        let local_repo = LocalRepo {
            gh_id: repo_id.clone(),
            local_dir: Some(local_dir.to_string()),
            owner,
            name,
        };

        db.upsert_local_repo(local_repo).map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })?;

        Ok(())
    }
}
