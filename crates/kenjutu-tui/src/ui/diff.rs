use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::app::{App, Focus};
use kenjutu_core::models::{DiffLineType, FileDiff};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Diff;

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if let Some(ref file) = app.files.get(app.selected_file) {
        let path = file
            .new_path
            .as_deref()
            .or(file.old_path.as_deref())
            .unwrap_or("(unknown)");
        format!(" {} ", path)
    } else {
        " Diff ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match &app.diff {
        None => {
            let msg = if app.files.is_empty() {
                "Select a commit to view its diff"
            } else {
                "No diff available"
            };
            let paragraph = Paragraph::new(msg).style(Style::default().fg(Color::DarkGray));
            frame.render_widget(paragraph, inner);
        }
        Some(file_diff) => {
            render_diff(frame, app, file_diff, inner);
        }
    }
}

fn render_diff(frame: &mut Frame, app: &App, file_diff: &FileDiff, area: Rect) {
    let visible_height = area.height as usize;

    // Build all lines
    let mut lines: Vec<Line> = Vec::new();
    for hunk in &file_diff.hunks {
        // Hunk header
        let header_line = Line::from(Span::styled(
            &hunk.header,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(header_line);

        // Diff lines
        for diff_line in &hunk.lines {
            let line = render_diff_line(diff_line, area.width);
            lines.push(line);
        }
    }

    // Apply scroll offset
    let total = lines.len();
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(app.diff_scroll)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(Text::from(visible_lines));
    frame.render_widget(paragraph, area);

    // Scrollbar
    if total > visible_height {
        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y,
            width: 1,
            height: area.height,
        };

        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(visible_height))
            .position(app.diff_scroll);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn render_diff_line(diff_line: &kenjutu_core::models::DiffLine, width: u16) -> Line<'static> {
    let (bg_color, line_prefix) = match diff_line.line_type {
        DiffLineType::Addition => (Color::Rgb(20, 40, 20), "+"),
        DiffLineType::Deletion => (Color::Rgb(50, 20, 20), "-"),
        DiffLineType::Context => (Color::Reset, " "),
        DiffLineType::AddEofnl => (Color::Rgb(20, 40, 20), "+"),
        DiffLineType::DelEofnl => (Color::Rgb(50, 20, 20), "-"),
    };

    // Line numbers
    let old_no = diff_line
        .old_lineno
        .map(|n| format!("{:>4}", n))
        .unwrap_or_else(|| "    ".to_string());
    let new_no = diff_line
        .new_lineno
        .map(|n| format!("{:>4}", n))
        .unwrap_or_else(|| "    ".to_string());

    let lineno_style = Style::default().fg(Color::DarkGray).bg(bg_color);
    let prefix_style = match diff_line.line_type {
        DiffLineType::Addition => Style::default().fg(Color::Green).bg(bg_color),
        DiffLineType::Deletion => Style::default().fg(Color::Red).bg(bg_color),
        _ => Style::default().fg(Color::DarkGray).bg(bg_color),
    };

    let mut spans: Vec<Span> = vec![
        Span::styled(old_no, lineno_style),
        Span::styled(" ", lineno_style),
        Span::styled(new_no, lineno_style),
        Span::styled(format!(" {} ", line_prefix), prefix_style),
    ];

    // Render tokens with syntax highlighting colors
    for token in &diff_line.tokens {
        let fg = token
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(Color::White);

        let token_bg = if token.changed {
            match diff_line.line_type {
                DiffLineType::Addition => Color::Rgb(40, 80, 40),
                DiffLineType::Deletion => Color::Rgb(90, 30, 30),
                _ => bg_color,
            }
        } else {
            bg_color
        };

        let style = Style::default().fg(fg).bg(token_bg);
        spans.push(Span::styled(token.content.clone(), style));
    }

    // Fill rest of line with background color
    let content_len: usize = 4 + 1 + 4 + 3 + diff_line
        .tokens
        .iter()
        .map(|t| t.content.len())
        .sum::<usize>();
    let remaining = (width as usize).saturating_sub(content_len);
    if remaining > 0 {
        spans.push(Span::styled(
            " ".repeat(remaining),
            Style::default().bg(bg_color),
        ));
    }

    Line::from(spans)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }
    let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
    let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
    let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
