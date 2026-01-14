use std::path::PathBuf;

use octocrab::Octocrab;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::collect_commands;

use crate::commands::{
    get_commit_diff, get_pull, get_pull_requests, get_repo_by_id, get_repositories,
    lookup_repository_node_id, set_local_repo, toggle_file_reviewed,
};
use crate::db::DB;
use crate::errors::CommandError;
use crate::services::GitHubService;

mod commands;
mod config;
mod db;
mod errors;
mod models;
mod services;

pub struct App {
    client: Octocrab,
    db_path: PathBuf,
}

impl App {
    fn new(client: Octocrab, data_dir: PathBuf) -> Self {
        let db_path = data_dir.join("pr.db");
        Self { client, db_path }
    }

    fn get_connection(&self) -> Result<DB, CommandError> {
        rusqlite::Connection::open(&self.db_path)
            .map(DB::new)
            .map_err(|err| {
                log::error!("failed to open sqlite: {err}");
                CommandError::Internal
            })
    }

    fn github_service(&self) -> GitHubService {
        GitHubService::new(self.client.clone())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tauri_specta::Builder::<tauri::Wry>::new().commands(collect_commands![
        get_repositories,
        lookup_repository_node_id,
        get_repo_by_id,
        set_local_repo,
        get_pull_requests,
        get_pull,
        get_commit_diff,
        toggle_file_reviewed,
    ]);

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
                .target(Target::new(TargetKind::LogDir { file_name: None }))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            log::info!("Application starting up - logging initialized");
            let token = config::load_token()?;
            let client = Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|err| format!("Failed to create client: {}", err))?;

            let app_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_dir)
                .map_err(|err| format!("Failed to create data directory: {}", err))?;

            let my_app = App::new(client, app_dir);
            app.manage(my_app);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_repositories,
            get_pull_requests,
            get_repo_by_id,
            set_local_repo,
            get_pull,
            get_commit_diff,
            toggle_file_reviewed
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
