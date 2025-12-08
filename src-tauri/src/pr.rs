use git2::Oid;
use serde::Serialize;
use specta::Type;
use tauri::{command, State};

use super::App;

#[derive(Clone, Debug, Type, Serialize)]
pub struct GetPullResponse {
    pub title: String,
    pub body: String,
    pub base_branch: String,
    pub head_branch: String,
    pub commits: Vec<PRCommit>,
}

#[derive(Clone, Debug, Type, Serialize)]
pub struct PRCommit {
    pub change_id: Option<String>,
    pub sha: String,
    pub summary: String,
    pub description: String,
}

#[command]
#[specta::specta]
pub async fn get_pull(
    app: State<'_, App>,
    owner: String,
    repo: String,
    pr: u64,
) -> Result<GetPullResponse, String> {
    let pr = app
        .client
        .pulls(&owner, &repo)
        .get(pr)
        .await
        .map_err(|err| {
            log::error!("failed to get pr: {err}");
            "Failed to fetch PR".to_string()
        })?;

    let gh_repo = app.client.repos(owner, repo).get().await.map_err(|err| {
        log::error!("Could not find repository {err}");
        "Failed to get repository"
    })?;

    let repo_node_id = gh_repo.node_id.ok_or_else(|| {
        log::error!("got null node id");
        "Internal Error"
    })?;

    let repo_dir = app
        .get_connection()
        .await?
        .find_local_repo(&repo_node_id)
        .await
        .map_err(|err| {
            log::error!("DB error: {err}");
            "Internal Error"
        })?
        .ok_or_else(|| "Please Set local Repository to review PR")?;

    let repository = git2::Repository::open(&repo_dir.local_dir).map_err(|err| {
        log::error!("couldn't find local repository: {err}");
        "Counld not connect to repository set by user. Please reset local repository for this repository"
    })?;

    let head_sha = Oid::from_str(&pr.head.sha).expect("Github gave me a wrong hash");
    let base_sha = Oid::from_str(&pr.base.sha).expect("Github gave me a wrong hash");

    let mut walker = repository.revwalk().map_err(|err| {
        log::error!("failed to intiate rev walker: {err}");
        "Internal Error"
    })?;
    let range = format!("{}..{}", base_sha, head_sha);
    walker.push_range(&range).map_err(|err| {
        log::error!("failed to push range to walker: {err}");
        "Internal Error"
    })?;

    let mut commits: Vec<PRCommit> = Vec::new();
    for oid in walker {
        let oid = oid.map_err(|err| {
            log::error!("what is this error {err}");
            "Internal Error"
        })?;
        let commit = repository.find_commit(oid).map_err(|err| {
            log::error!("this really should not happed {err}");
            "Internal Error"
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
        title: pr.title.unwrap_or("".to_string()),
        body: pr.body.unwrap_or("".to_string()),
        base_branch: pr.base.ref_field,
        head_branch: pr.head.ref_field,
        commits,
    })
}
