use std::sync::Mutex;

use git2::Repository;
use tauri::{command, Manager, State};

#[command]
fn set_repository(app: State<'_, Mutex<App>>, dir: String) -> Result<(), String> {
    let repo = Repository::open(dir).map_err(|_| "failed to load reposotory".to_string())?;
    {
        let mut app = app.lock().unwrap();
        app.repository = Some(repo);
    }
    Ok(())
}

struct App {
    repository: Option<Repository>,
}

impl App {
    fn new() -> Self {
        Self { repository: None }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(Mutex::new(App::new()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_repository])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
