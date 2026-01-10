use std::path::PathBuf;

use octocrab::Octocrab;
use specta_typescript::Typescript;
use sqlx::SqlitePool;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_specta::collect_commands;

use crate::commands::{
    get_commit_diff, get_pull, get_pull_requests, get_repo_by_id, get_repositories, set_local_repo,
    toggle_file_reviewed,
};
use crate::db::DB;
use crate::errors::CommandError;
use crate::services::GitHubService;
use crate::state::AppState;

mod commands;
mod config;
mod db;
mod errors;
mod models;
mod services;
mod state;

pub struct App {
    client: Octocrab,
    pool: SqlitePool,
}

impl App {
    async fn new(client: Octocrab, data_dir: PathBuf) -> Result<Self, String> {
        let db_path = data_dir.join("pr.db");
        let db_url = format!("sqlite:///{}", db_path.to_str().unwrap());

        let pool = SqlitePool::connect(&db_url)
            .await
            .map_err(|err| format!("Failed to connect to database: {}", err))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|err| format!("Failed to run migrations: {}", err))?;

        Ok(Self { client, pool })
    }

    async fn get_connection(&self) -> Result<DB, CommandError> {
        self.pool.acquire().await.map(DB::new).map_err(|err| {
            log::error!("Failed to get connection from pool: {err}");
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
        get_pull_requests,
        get_repo_by_id,
        set_local_repo,
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

            let app_state = AppState::new();
            app.manage(app_state);

            log::info!("Starting async application state initialization...");
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match App::new(client, app_dir).await {
                    Ok(app_instance) => {
                        if let Err(_) = app_handle.state::<AppState>().set(app_instance) {
                            log::error!("Failed to set application state - already initialized?");
                            return;
                        }
                        log::info!(
                            "Application state initialized successfully - ready for commands"
                        );
                    }
                    Err(err) => {
                        log::error!("FATAL: Failed to initialize application state: {}", err);
                        std::process::exit(1);
                    }
                }
            });

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
