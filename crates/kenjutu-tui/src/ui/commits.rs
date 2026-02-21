use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, Focus};


pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Side;

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Commits ({}) ", app.commits.len()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.commits.is_empty() {
        let msg = Line::from("No commits found").style(Style::default().fg(Color::DarkGray));
        let paragraph = ratatui::widgets::Paragraph::new(msg);
        frame.render_widget(paragraph, inner);
        return;
    }

    // Determine visible range with scrolling
    let visible_height = inner.height as usize;
    let total = app.commits.len();
    let scroll_offset = compute_scroll_offset(app.selected_commit, visible_height, total);

    let items: Vec<ListItem> = app
        .commits
        .iter()
        .zip(app.graph_rows.iter())
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, (commit, row))| {
            let graph = render_graph_prefix(row);
            let selected = idx == app.selected_commit;

            let short_id_style = Style::default().fg(Color::Cyan);
            let summary_style = if selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(vec![
                Span::raw(graph),
                Span::styled(commit.short_id.clone(), short_id_style),
                Span::raw(" "),
                Span::styled(commit.summary.clone(), summary_style),
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

fn render_graph_prefix(row: &crate::data::GraphRow) -> String {
    if row.num_lanes == 0 {
        return "* ".to_string();
    }

    let mut chars = vec![' '; row.num_lanes * 2];

    // Draw continuing lanes
    for i in 0..row.num_lanes {
        // If there's a continuing lane here, draw a pipe
        for &(from, to) in &row.edges {
            if from == i || to == i {
                chars[i * 2] = '|';
            }
        }
    }

    // Draw the commit node
    if row.col * 2 < chars.len() {
        chars[row.col * 2] = '*';
    }

    // Draw merge edges
    for &(from, to) in &row.edges {
        if from != to {
            let (left, right) = if from < to { (from, to) } else { (to, from) };
            for i in (left * 2 + 1)..=(right * 2) {
                if chars[i] == ' ' {
                    if i == left * 2 + 1 {
                        chars[i] = if from < to { '\\' } else { '/' };
                    } else if i == right * 2 {
                        chars[i] = if from < to { '\\' } else { '/' };
                    } else {
                        chars[i] = '-';
                    }
                }
            }
        }
    }

    let s: String = chars.into_iter().collect();
    format!("{} ", s.trim_end())
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
