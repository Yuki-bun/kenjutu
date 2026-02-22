mod commits;
mod diff;
mod files;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, Focus, SidePanel};

pub fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    // Split into side pane (30%) and diff pane (70%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);

    let side_area = chunks[0];
    let diff_area = chunks[1];

    // Side pane: tabs + content
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(side_area);

    draw_side_tabs(frame, app, side_chunks[0]);

    match app.side_panel {
        SidePanel::Commits => commits::draw(frame, app, side_chunks[1]),
        SidePanel::Files => files::draw(frame, app, side_chunks[1]),
    }

    // Diff pane
    diff::draw(frame, app, diff_area);

    // Status bar
    draw_status_bar(frame, app, status_area);
}

fn draw_side_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Commits [1]", "Files [2]"];
    let selected = match app.side_panel {
        SidePanel::Commits => 0,
        SidePanel::Files => 1,
    };

    let highlight_style = if app.focus == Focus::Side {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Side"))
        .select(selected)
        .highlight_style(highlight_style);

    frame.render_widget(tabs, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let focus_indicator = match app.focus {
        Focus::Side => "SIDE",
        Focus::Diff => "DIFF",
    };

    let commit_info = app
        .commits
        .get(app.selected_commit)
        .map(|c| format!("{} {}", c.short_id, c.summary.chars().take(40).collect::<String>()))
        .unwrap_or_default();

    let file_info = if !app.files.is_empty() {
        let f = &app.files[app.selected_file];
        f.new_path
            .as_deref()
            .or(f.old_path.as_deref())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    let status_text = if let Some(ref err) = app.error_msg {
        format!(" [{}] ERROR: {}", focus_indicator, err)
    } else {
        format!(
            " [{}] {} | {} | q:quit Tab:switch j/k:navigate Enter:select",
            focus_indicator, commit_info, file_info
        )
    };

    let style = if app.error_msg.is_some() {
        Style::default().bg(Color::Red).fg(Color::White)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    let bar = Paragraph::new(status_text).style(style);
    frame.render_widget(bar, area);
}
