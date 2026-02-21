use tauri::Manager;

use crate::commands::{
    auth_github, get_change_id_from_sha, get_commit_file_list, get_commits_in_range,
    get_context_lines, get_file_diff, get_jj_log, get_jj_status, get_partial_review_diffs,
    mark_hunk_reviewed, toggle_file_reviewed, unmark_hunk_reviewed, validate_git_repo,
};

mod commands;
mod models;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, argv, _cwd| {
            println!("a new app instance was opened with {argv:?}");
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            log::info!("Application starting up - logging initialized");
            let app_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_dir)
                .map_err(|err| format!("Failed to create data directory: {}", err))?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth_github,
            get_change_id_from_sha,
            get_commit_file_list,
            get_commits_in_range,
            get_context_lines,
            get_file_diff,
            get_jj_log,
            get_jj_status,
            get_partial_review_diffs,
            mark_hunk_reviewed,
            toggle_file_reviewed,
            unmark_hunk_reviewed,
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
            get_context_lines,
            get_file_diff,
            get_jj_log,
            get_jj_status,
            get_partial_review_diffs,
            mark_hunk_reviewed,
            toggle_file_reviewed,
            unmark_hunk_reviewed,
            validate_git_repo,
        ])
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            "./src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");
}
