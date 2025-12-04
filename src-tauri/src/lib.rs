use std::env;
use std::fs::File;
use std::path::Path;

use octocrab::{params, Octocrab};
use serde::Serialize;

use specta::Type;
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::{command, Manager, State};
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::collect_commands;

struct App {
    client: Octocrab,
}

#[derive(Type, Serialize, Debug, Clone)]
pub struct Repo {
    pub name: String,
    pub html_url: String,
    pub owner_name: String,
}

impl From<octocrab::models::Repository> for Repo {
    fn from(value: octocrab::models::Repository) -> Self {
        Self {
            name: value.name,
            html_url: value.html_url.map(|u| u.to_string()).unwrap_or_default(),
            owner_name: value.owner.map(|o| o.login).unwrap_or_default(),
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
    fn new(github_token: String) -> Result<Self, String> {
        let client = Octocrab::builder()
            .personal_token(github_token)
            .build()
            .map_err(|err| format!("failed to create client: {}", err))?;
        Ok(Self { client })
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
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![get_reposiotires, get_pull_requests,]);

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
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
            log::error!("Application starting up - logging initialized");
            let token = load_token()?;
            app.manage(App::new(token)?);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_reposiotires,
            get_pull_requests
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
