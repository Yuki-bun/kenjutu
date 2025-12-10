use octocrab::Octocrab;

use crate::errors::{CommandError, Result};

pub struct GitHubService {
    client: Octocrab,
}

impl GitHubService {
    pub fn new(client: Octocrab) -> Self {
        Self { client }
    }

    pub async fn list_repositories(&self) -> Result<Vec<octocrab::models::Repository>> {
        self.client
            .current()
            .list_repos_for_authenticated_user()
            .visibility("all")
            .sort("updated")
            .per_page(100)
            .send()
            .await
            .map(|page| page.into_iter().collect())
            .map_err(|err| {
                log::error!("Failed to fetch repositories: {}", err);
                CommandError::Internal
            })
    }

    pub async fn get_repository(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<octocrab::models::Repository> {
        self.client.repos(owner, name).get().await.map_err(|err| {
            log::error!("Failed to fetch repository: {}", err);
            CommandError::Internal
        })
    }

    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<octocrab::models::pulls::PullRequest>> {
        self.client
            .pulls(owner, repo)
            .list()
            .state(octocrab::params::State::Open)
            .sort(octocrab::params::pulls::Sort::Updated)
            .page(0_u32)
            .send()
            .await
            .map(|mut page| page.take_items())
            .map_err(|err| {
                log::error!("Failed to get pull requests: {}", err);
                CommandError::Internal
            })
    }

    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<octocrab::models::pulls::PullRequest> {
        self.client
            .pulls(owner, repo)
            .get(number)
            .await
            .map_err(|err| {
                log::error!("Failed to get pull request: {}", err);
                CommandError::Internal
            })
    }
}
