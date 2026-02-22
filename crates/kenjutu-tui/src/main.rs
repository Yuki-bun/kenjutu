mod app;
mod data;
mod ui;

use std::io;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

use app::{App, Focus, SidePanel};

#[derive(Parser)]
#[command(name = "kenjutu-tui", about = "TUI diff viewer for git repositories")]
struct Cli {
    /// Path to the git repository
    #[arg(default_value = ".")]
    path: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &cli.path);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, repo_path: &str) -> Result<()> {
    let mut app = App::new(repo_path)?;

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keybindings
            match (key.modifiers, key.code) {
                (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    return Ok(());
                }
                (_, KeyCode::Tab) => {
                    app.cycle_focus();
                    continue;
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    app.cycle_focus_back();
                    continue;
                }
                _ => {}
            }

            // Panel-specific keybindings
            match app.focus {
                Focus::Side => match key.code {
                    KeyCode::Char('1') => app.side_panel = SidePanel::Commits,
                    KeyCode::Char('2') => app.side_panel = SidePanel::Files,
                    KeyCode::Up | KeyCode::Char('k') => app.side_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.side_down(),
                    KeyCode::Enter => app.side_select(),
                    _ => {}
                },
                Focus::Diff => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.diff_scroll_up(1),
                    KeyCode::Down | KeyCode::Char('j') => app.diff_scroll_down(1),
                    KeyCode::PageUp | KeyCode::Char('b') => app.diff_scroll_up(20),
                    KeyCode::PageDown | KeyCode::Char('f') => app.diff_scroll_down(20),
                    KeyCode::Home | KeyCode::Char('g') => app.diff_scroll = 0,
                    KeyCode::End | KeyCode::Char('G') => app.diff_scroll_to_end(),
                    _ => {}
                },
            }
        }
    }
}
