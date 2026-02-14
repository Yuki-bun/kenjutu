use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::commands::{
    auth_github, get_change_id_from_sha, get_commit_file_list, get_commits_in_range, get_file_diff,
    get_jj_log, get_jj_status, toggle_file_reviewed, validate_git_repo,
};

mod commands;
mod db;
mod models;
mod services;

#[cfg(test)]
mod test_utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
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
        .plugin(tauri_plugin_store::Builder::new().build())
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

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            auth_github,
            get_change_id_from_sha,
            get_commit_file_list,
            get_commits_in_range,
            get_file_diff,
            get_jj_log,
            get_jj_status,
            toggle_file_reviewed,
            validate_git_repo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

pub fn gen_ts_bindings() {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            auth_github,
            get_change_id_from_sha,
            get_commit_file_list,
            get_commits_in_range,
            get_file_diff,
            get_jj_log,
            get_jj_status,
            toggle_file_reviewed,
            validate_git_repo,
        ])
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            "./src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");
}
