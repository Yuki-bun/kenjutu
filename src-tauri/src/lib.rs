use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fmt::Debug};

use octocrab::{params, Octocrab};
use serde::Serialize;

use specta::Type;
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use sqlx::SqlitePool;
use tauri::{command, Manager, State};
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::collect_commands;

use crate::db::{LocalRepo, DB};

mod commands;
mod db;
mod github;

struct App {
    client: Octocrab,
    pool: SqlitePool,
}

#[derive(Type, Serialize, Debug, Clone)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub html_url: String,
    pub owner_name: String,
}

impl From<octocrab::models::Repository> for Repo {
    fn from(value: octocrab::models::Repository) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            html_url: value.html_url.map(|u| u.to_string()).unwrap_or_default(),
            owner_name: value.owner.map(|o| o.login).unwrap_or_default(),
        }
    }
}

#[derive(Type, Serialize, Debug, Clone)]
pub struct FullRepo {
    pub name: String,
    pub owner_name: Option<String>,
    pub description: Option<String>,
    pub local_repo: Option<PathBuf>,
}

impl FullRepo {
    fn new(octo_repo: octocrab::models::Repository, local_repo: Option<PathBuf>) -> Self {
        Self {
            name: octo_repo.name,
            owner_name: octo_repo.owner.and_then(|owner| owner.name),
            description: octo_repo.description,
            local_repo,
        }
    }
}

#[command]
#[specta::specta]
async fn get_reposiotires(app: State<'_, App>) -> Result<Vec<Repo>, String> {
    let repos = app
        .client
        .current()
        .list_repos_for_authenticated_user()
        .visibility("all")
        .sort("updated")
        .per_page(100)
        .send()
        .await
        .map_err(|err| format!("failed to fetch repos: {}", err))?
        .into_iter()
        .map(Repo::from)
        .collect();
    Ok(repos)
}

#[command]
#[specta::specta]
async fn set_local_repo(
    app: State<'_, App>,
    owner: String,
    name: String,
    local_dir: String,
) -> Result<(), String> {
    if git2::Repository::open(&local_dir).is_err() {
        return Err(format!("directory {} is not a git repository", local_dir));
    }
    let repo = app
        .client
        .repos(owner, name)
        .get()
        .await
        .map_err(|_| "github repository not found".to_string())?;

    let mut db = app.get_connection().await?;

    let local_repo = LocalRepo {
        local_dir,
        github_id: repo.id.0 as i64,
    };
    db.upsert_local_repo(local_repo).await.map_err(|err| {
        log::error!("db errored: {err}");
        "Internal Error".to_string()
    })?;

    Ok(())
}

#[command]
#[specta::specta]
async fn get_repo_by_id(
    app: State<'_, App>,
    owner: String,
    name: String,
) -> Result<FullRepo, String> {
    let repo = app.client.repos(owner, name).get().await.map_err(|err| {
        log::error!("githubApi error: {}", err);
        "Failed to Connect to github api".to_string()
    })?;
    let mut db = app.get_connection().await?;
    let local_dir = db.find_local_repo(repo.id.0).await.map_or_else(
        |err| {
            log::error!("db errored {err}");
            None
        },
        |repo| repo.map(|repo| PathBuf::from(repo.local_dir)),
    );
    Ok(FullRepo::new(repo, local_dir))
}

#[derive(Type, Serialize, Debug, Clone)]
pub struct PullRequest {
    github_url: Option<String>,
    id: u32,
    title: Option<String>,
    author: Option<User>,
}

impl From<octocrab::models::pulls::PullRequest> for PullRequest {
    fn from(value: octocrab::models::pulls::PullRequest) -> Self {
        Self {
            github_url: value.html_url.map(|url| url.into()),
            id: value.id.0 as u32,
            title: value.title,
            author: value.user.map(|owner| User::from(*owner)),
        }
    }
}

#[derive(Type, Serialize, Debug, Clone)]
pub struct User {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub name: Option<String>,
}

impl From<octocrab::models::Author> for User {
    fn from(value: octocrab::models::Author) -> Self {
        Self {
            login: value.login,
            id: value.id.0 as u32,
            avatar_url: value.avatar_url.into(),
            gravatar_id: value.gravatar_id,
            name: value.name,
        }
    }
}

#[command]
#[specta::specta]
async fn get_pull_requests(
    app: State<'_, App>,
    owner: String,
    repo: String,
) -> Result<Vec<PullRequest>, String> {
    let page = app
        .client
        .pulls(owner, repo)
        .list()
        .state(params::State::Open)
        .sort(params::pulls::Sort::Updated)
        .page(0 as u32)
        .send()
        .await
        .map_err(|err| format!("failed to get prs {}", err))?
        .take_items()
        .into_iter()
        .map(PullRequest::from)
        .collect();

    Ok(page)
}

impl App {
    fn new(client: Octocrab, pool: SqlitePool) -> Result<Self, String> {
        Ok(Self { client, pool })
    }

    async fn get_connection(&self) -> Result<DB, String> {
        let conn = self.pool.acquire().await.map_err(|err| {
            log::error!("failed to establish connection: {}", err);
            "Internal Error".to_string()
        })?;
        Ok(DB::new(conn))
    }
}

fn load_token() -> Result<String, Box<dyn std::error::Error>> {
    let token_path = Path::new("/home/mech-user/.config/pr-manager/token");
    let file =
        File::open(token_path).map_err(|err| format!("failed to open toke file: {}", err))?;
    let token = std::io::read_to_string(file)?;
    Ok(token.trim().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tauri_specta::Builder::<tauri::Wry>::new().commands(collect_commands![
        get_reposiotires,
        get_pull_requests,
        get_repo_by_id,
        set_local_repo,
    ]);

    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default().bigint(specta_typescript::BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .target(Target::new(TargetKind::LogDir {
                    //file_name: Some("/home/mech-user/programming/pr-manager/debug".into()),
                    file_name: None,
                }))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            log::info!("Application starting up - logging initialized");
            let token = load_token()?;
            let client = Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|err| format!("failed to create client: {}", err))?;

            app.manage(App::new(client, pool)?);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_reposiotires,
            get_pull_requests,
            get_repo_by_id,
            set_local_repo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
