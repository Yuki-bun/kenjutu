use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use oauth2::AccessToken;
use octocrab::Octocrab;
use rustls::lock::Mutex;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::commands::{
    auth_github, get_commit_diff, get_pull, get_pull_requests, get_repo_by_id, get_repositories,
    lookup_repository_node_id, merge_pull_request, set_local_repo, toggle_file_reviewed,
};
use crate::db::DB;
use crate::errors::CommandError;
use crate::services::GitHubService;

mod commands;
mod db;
mod errors;
mod models;
mod services;

#[derive(Debug)]
pub struct App {
    client: Mutex<Octocrab>,
    db_path: PathBuf,
}

impl App {
    fn new(app_data_dir: PathBuf) -> Self {
        let token_path = app_data_dir.join("token");
        let token = fs::read_to_string(token_path).ok();
        let mut client_builder = octocrab::OctocrabBuilder::new();
        if let Some(token) = token {
            log::info!("Using token from token file");
            client_builder = client_builder.user_access_token(token);
        } else {
            log::info!("Token file was not found");
        }

        let client = client_builder.build().expect("Should build client");

        let db_path = app_data_dir.join("pr.db");
        Self {
            client: Mutex::new(client),
            db_path,
        }
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
        GitHubService::new(self.client.lock().unwrap().clone())
    }

    fn set_access_token(&self, token: AccessToken) {
        let new_client = {
            let guard = self.client.lock().unwrap();
            guard.user_access_token(token.secret().to_owned())
        };
        match new_client {
            Ok(new_client) => {
                *self.client.lock().unwrap() = new_client;
            }
            Err(err) => {
                log::error!("Faild to set access token: {:?}", err)
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    gen_ts_bindings();

    let mut builder = tauri::Builder::default().plugin(tauri_plugin_deep_link::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, argv, _cwd| {
          println!("a new app instance was opened with {argv:?} and the deep link event was already triggered");
          // when defining deep link schemes at runtime, you must also check `argv` here
          // 実行時に「deep link」スキーム（設定構成）を定義する場合は、ここで `argv` も確認する必要があります。
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .target(Target::new(TargetKind::LogDir { file_name: None }))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            log::info!("Application starting up - logging initialized");
            let app_dir = app.path().app_data_dir()?;

            #[cfg(any(windows, target_os = "linux"))]
            {
                log::info!("Setting up deep link");
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            std::fs::create_dir_all(&app_dir)
                .map_err(|err| format!("Failed to create data directory: {}", err))?;

            let my_app = App::new(app_dir);
            app.manage(Arc::new(my_app));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            auth_github,
            get_commit_diff,
            get_pull,
            get_pull_requests,
            get_repo_by_id,
            get_repositories,
            lookup_repository_node_id,
            merge_pull_request,
            set_local_repo,
            toggle_file_reviewed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

#[cfg(debug_assertions)]
fn gen_ts_bindings() {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            auth_github,
            get_commit_diff,
            get_pull,
            get_pull_requests,
            get_repo_by_id,
            get_repositories,
            lookup_repository_node_id,
            merge_pull_request,
            set_local_repo,
            toggle_file_reviewed,
        ])
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");
}
