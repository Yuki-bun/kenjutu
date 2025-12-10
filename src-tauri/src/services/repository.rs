use std::path::PathBuf;

use crate::db::{LocalRepo, DB};
use crate::errors::{CommandError, Result};
use crate::models::{FullRepo, Repo};
use crate::services::GitHubService;

pub struct RepositoryService;

impl RepositoryService {
    pub async fn get_repositories(github: &GitHubService) -> Result<Vec<Repo>> {
        let repos = github.list_repositories().await?;
        Ok(repos.into_iter().map(Repo::from).collect())
    }

    pub async fn get_repository_details(
        github: &GitHubService,
        db: &mut DB,
        owner: &str,
        name: &str,
    ) -> Result<FullRepo> {
        let repo = github.get_repository(owner, name).await?;

        let github_node_id = repo.node_id.as_ref().ok_or_else(|| {
            log::error!("Found repo that does not have node_id. owner: {owner}, name: {name}");
            CommandError::Internal
        })?;

        let local_dir = db.find_local_repo(github_node_id).await.map_or_else(
            |err| {
                log::error!("DB error: {err}");
                None
            },
            |repo| repo.map(|repo| PathBuf::from(repo.local_dir)),
        );

        Ok(FullRepo::new(repo, local_dir))
    }

    pub async fn set_local_repository(
        github: &GitHubService,
        db: &mut DB,
        owner: &str,
        name: &str,
        local_dir: &str,
    ) -> Result<()> {
        // Validate git repository
        if git2::Repository::open(local_dir).is_err() {
            return Err(CommandError::bad_input(format!(
                "Directory {} is not a git repository",
                local_dir
            )));
        }

        let repo = github.get_repository(owner, name).await?;

        let github_node_id = repo.node_id.ok_or_else(|| {
            log::error!("Found repo that does not have node_id. owner: {owner}, name: {name}");
            CommandError::Internal
        })?;

        let local_repo = LocalRepo {
            local_dir: local_dir.to_string(),
            github_node_id,
        };

        db.upsert_local_repo(local_repo).await.map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })?;

        Ok(())
    }
}
