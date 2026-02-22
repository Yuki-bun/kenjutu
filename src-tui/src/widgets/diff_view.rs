use kenjutu_core::models::{DiffLineType, FileDiff};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use crate::color::css_hex_to_color;

pub struct DiffViewWidget<'a> {
    diff: Option<&'a FileDiff>,
    scroll_offset: usize,
    block: Option<Block<'a>>,
}

impl<'a> DiffViewWidget<'a> {
    pub fn new(diff: Option<&'a FileDiff>, scroll_offset: usize) -> Self {
        Self {
            diff,
            scroll_offset,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    fn build_lines(diff: &FileDiff) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        for hunk in &diff.hunks {
            // Hunk header
            lines.push(Line::from(Span::styled(
                &hunk.header,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            )));

            for diff_line in &hunk.lines {
                let mut spans = Vec::new();

                // Line numbers gutter
                let old_no = diff_line
                    .old_lineno
                    .map(|n| format!("{:>4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                let new_no = diff_line
                    .new_lineno
                    .map(|n| format!("{:>4}", n))
                    .unwrap_or_else(|| "    ".to_string());

                spans.push(Span::styled(
                    format!("{} {} ", old_no, new_no),
                    Style::default().fg(Color::DarkGray),
                ));

                // Line type prefix and base style
                let (prefix, line_bg, changed_bg) = match diff_line.line_type {
                    DiffLineType::Addition => ("+", Color::Rgb(0, 40, 0), Color::Rgb(0, 80, 0)),
                    DiffLineType::Deletion => ("-", Color::Rgb(40, 0, 0), Color::Rgb(80, 0, 0)),
                    DiffLineType::Context => (" ", Color::Reset, Color::Reset),
                    DiffLineType::AddEofnl | DiffLineType::DelEofnl => {
                        ("\\ ", Color::Reset, Color::Reset)
                    }
                };

                spans.push(Span::styled(prefix, Style::default().bg(line_bg)));

                // Syntax-highlighted tokens
                for token in &diff_line.tokens {
                    let fg = token
                        .color
                        .as_deref()
                        .and_then(css_hex_to_color)
                        .unwrap_or(Color::White);

                    let bg = if token.changed { changed_bg } else { line_bg };

                    let mut style = Style::default().fg(fg).bg(bg);
                    if token.changed {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }

                    spans.push(Span::styled(&token.content, style));
                }

                lines.push(Line::from(spans));
            }
        }

        lines
    }
}

impl<'a> Widget for DiffViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = match self.diff {
            Some(diff) => Self::build_lines(diff),
            None => vec![Line::from(Span::styled(
                "No file selected",
                Style::default().fg(Color::DarkGray),
            ))],
        };

        let mut paragraph = Paragraph::new(lines).scroll((self.scroll_offset as u16, 0));

        if let Some(block) = self.block {
            paragraph = paragraph.block(block);
        }

        paragraph.render(area, buf);
    }
}
