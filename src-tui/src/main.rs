use std::fs::File;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use kenjutu_core::services::{git, jj};
use kenjutu_tui::app::App;
use kenjutu_tui::error;
use kenjutu_tui::tui;
use log::LevelFilter;

#[derive(Parser)]
#[command(name = "kenjutu", about = "TUI code review tool for jj repositories")]
struct Cli {
    /// Path to the repository directory
    #[arg(short, long, default_value = ".")]
    dir: String,
}

fn init_logging() {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        .join("kenjutu-tui.log");

    let log_file = File::create(&log_path).expect("failed to create log file");

    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .format_timestamp_millis()
        .init();

    log::info!("logging initialized to {}", log_path.display());
}

fn main() -> error::Result<()> {
    init_logging();

    let cli = Cli::parse();

    let local_dir = std::fs::canonicalize(&cli.dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| cli.dir.clone());

    log::info!("starting kenjutu-tui for {}", local_dir);

    if !jj::is_jj_repo(&local_dir) {
        eprintln!("Error: {} is not a jj repository", local_dir);
        std::process::exit(1);
    }

    let repository = git::open_repository(&local_dir)?;
    let mut terminal = tui::setup_terminal()?;
    let mut app = App::new(local_dir, repository);

    app.load_initial_commits();
    log::info!("loaded {} commits", app.commit_log.commits.len());

    loop {
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    log::info!(
                        "key={:?} modifiers={:?} screen={}",
                        key.code,
                        key.modifiers,
                        app.screen_name()
                    );
                    app.handle_key_event(key);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    log::info!("shutting down");
    tui::restore_terminal(&mut terminal)?;
    Ok(())
}
