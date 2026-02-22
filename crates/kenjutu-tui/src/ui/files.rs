use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, Focus};
use kenjutu_core::models::FileChangeStatus;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Side;

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let commit_info = app
        .commits
        .get(app.selected_commit)
        .map(|c| c.short_id.as_str())
        .unwrap_or("");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(
            " Files ({}) [{}] ",
            app.files.len(),
            commit_info
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.files.is_empty() {
        let msg = Line::from("No files changed").style(Style::default().fg(Color::DarkGray));
        let paragraph = ratatui::widgets::Paragraph::new(msg);
        frame.render_widget(paragraph, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let total = app.files.len();
    let scroll_offset = compute_scroll_offset(app.selected_file, visible_height, total);

    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, file)| {
            let selected = idx == app.selected_file;

            let (status_char, status_color) = match file.status {
                FileChangeStatus::Added => ('A', Color::Green),
                FileChangeStatus::Modified => ('M', Color::Yellow),
                FileChangeStatus::Deleted => ('D', Color::Red),
                FileChangeStatus::Renamed => ('R', Color::Cyan),
                FileChangeStatus::Copied => ('C', Color::Blue),
                FileChangeStatus::Typechange => ('T', Color::Magenta),
            };

            let path = file
                .new_path
                .as_deref()
                .or(file.old_path.as_deref())
                .unwrap_or("(unknown)");

            let rename_suffix = if file.status == FileChangeStatus::Renamed {
                file.old_path
                    .as_ref()
                    .map(|old| format!(" (from {})", old))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let stats = format!("+{} -{}", file.additions, file.deletions);

            let path_style = if selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", status_char),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(path, path_style),
                Span::styled(rename_suffix, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(stats, Style::default().fg(Color::DarkGray)),
            ]);

            let item = ListItem::new(line);
            if selected {
                item.style(Style::default().bg(Color::Rgb(40, 40, 60)))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn compute_scroll_offset(selected: usize, visible: usize, total: usize) -> usize {
    if total <= visible {
        return 0;
    }
    if selected < visible / 2 {
        return 0;
    }
    let max_offset = total.saturating_sub(visible);
    let offset = selected.saturating_sub(visible / 2);
    offset.min(max_offset)
}
