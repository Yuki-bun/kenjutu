use std::env;
use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::{command, Manager, State};
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::collect_commands;

struct App {
    github_token: String,
}

#[derive(Serialize, Deserialize, Type)]
struct Repo {
    name: String,
    url: String,
}

#[command]
#[specta::specta]
async fn get_reposiotires(app: State<'_, App>) -> Result<Vec<Repo>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user/repos")
        .header("User-Agent", "Ferocious Review")
        .header("Authorization", format!("Bearer {}", app.github_token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .query(&[
            ("type", "owner"),
            ("sort", "updated"),
            ("direction", "desc"),
            ("per_page", "100"),
        ])
        .send()
        .await
        .map_err(|err| {
            log::error!("GitHub API request error: {:?}", err);
            format!("failed to send request: {err}")
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("GitHub API error {}: {}", status, body);
        return Err(format!("GitHub API error {}: {}", status, body));
    }

    let github_repos: Vec<Repo> = response.json().await.map_err(|err| {
        log::error!("Failed to parse response: {:?}", err);
        format!("failed to parse response: {err}")
    })?;

    Ok(github_repos)
}

impl App {
    fn new(github_token: String) -> Self {
        Self { github_token }
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
    let builder =
        tauri_specta::Builder::<tauri::Wry>::new().commands(collect_commands![get_reposiotires,]);

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Trace)
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
            app.manage(App::new(token));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_reposiotires])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
