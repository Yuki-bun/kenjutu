use crate::db::DB;
use crate::errors::{CommandError, Result};
use crate::services::GitHubService;

pub struct RepositoryCacheService;

impl RepositoryCacheService {
    /// Get repository metadata (owner, name) from node_id
    /// Tries cache first, fetches from GitHub REST API on miss
    pub async fn get_repo_owner_name(
        github: &GitHubService,
        db: &mut DB,
        node_id: &str,
    ) -> Result<(String, String)> {
        if let Some(repo) = db.find_repository(node_id).map_err(|err| {
            log::error!("DB error: {err}");
            CommandError::Internal
        })? {
            return Ok((repo.owner, repo.name));
        }

        log::warn!(
            "Cache miss for node_id: {}, fetching from GitHub REST API",
            node_id
        );
        let repos = github.list_repositories().await?;
        let repo = repos
            .into_iter()
            .find(|r| r.node_id.as_ref() == Some(&node_id.to_string()))
            .ok_or_else(|| {
                log::error!("Repository with node_id {node_id} not found");
                CommandError::bad_input("Repository not found")
            })?;

        let owner = repo
            .owner
            .as_ref()
            .map(|o| o.login.clone())
            .ok_or(CommandError::Internal)?;
        let name = repo.name.clone();

        db.upsert_repository_cache(node_id, &owner, &name)
            .map_err(|err| {
                log::error!("Failed to update cache: {err}");
                CommandError::Internal
            })?;

        Ok((owner, name))
    }

    /// Lookup node_id by owner/name
    /// Tries cache first, fetches from GitHub REST API on miss
    pub async fn lookup_node_id_by_owner_name(
        github: &GitHubService,
        db: &mut DB,
        owner: &str,
        name: &str,
    ) -> Result<String> {
        if let Some(repo) = db
            .find_repository_by_owner_name(owner, name)
            .map_err(|err| {
                log::error!("DB error: {err}");
                CommandError::Internal
            })?
        {
            return Ok(repo.github_node_id);
        }

        let repo = github.get_repository(owner, name).await?;
        let node_id = repo.node_id.ok_or_else(|| {
            log::error!("Repository has no node_id");
            CommandError::Internal
        })?;

        db.upsert_repository_cache(&node_id, owner, name)
            .map_err(|err| {
                log::error!("Failed to update cache: {err}");
                CommandError::Internal
            })?;

        Ok(node_id)
    }
}
