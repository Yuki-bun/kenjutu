use kenjutu_core::models::{DiffLineType, FileDiff};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};

use crate::color::css_hex_to_color;

pub struct DiffViewWidget<'a> {
    diff: Option<&'a FileDiff>,
    scroll_offset: usize,
    cursor_line: Option<usize>,
    selection: Option<(usize, usize)>,
    block: Option<Block<'a>>,
}

impl<'a> DiffViewWidget<'a> {
    pub fn new(diff: Option<&'a FileDiff>, scroll_offset: usize) -> Self {
        Self {
            diff,
            scroll_offset,
            cursor_line: None,
            selection: None,
            block: None,
        }
    }

    pub fn cursor_line(mut self, line: usize) -> Self {
        self.cursor_line = Some(line);
        self
    }

    pub fn selection(mut self, range: (usize, usize)) -> Self {
        self.selection = Some(range);
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    fn build_lines(
        diff: &FileDiff,
        cursor_line: Option<usize>,
        selection: Option<(usize, usize)>,
    ) -> Vec<Line<'_>> {
        let mut lines = Vec::new();
        let mut line_idx: usize = 0;

        for hunk in &diff.hunks {
            // Hunk header
            let header_highlight = Self::line_highlight(line_idx, cursor_line, selection);
            let mut header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM);
            if let Some(bg) = header_highlight {
                header_style = header_style.bg(bg);
            }
            lines.push(Line::from(Span::styled(&hunk.header, header_style)));
            line_idx += 1;

            for diff_line in &hunk.lines {
                let highlight_bg = Self::line_highlight(line_idx, cursor_line, selection);
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

                let mut gutter_style = Style::default().fg(Color::DarkGray);
                if let Some(bg) = highlight_bg {
                    gutter_style = gutter_style.bg(bg);
                }
                spans.push(Span::styled(
                    format!("{} {} ", old_no, new_no),
                    gutter_style,
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

                let effective_bg = highlight_bg.unwrap_or(line_bg);
                spans.push(Span::styled(prefix, Style::default().bg(effective_bg)));

                // Syntax-highlighted tokens
                for token in &diff_line.tokens {
                    let fg = token
                        .color
                        .as_deref()
                        .and_then(css_hex_to_color)
                        .unwrap_or(Color::White);

                    let bg = if token.changed {
                        changed_bg
                    } else {
                        highlight_bg.unwrap_or(line_bg)
                    };

                    let mut style = Style::default().fg(fg).bg(bg);
                    if token.changed {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }

                    spans.push(Span::styled(&token.content, style));
                }

                lines.push(Line::from(spans));
                line_idx += 1;
            }
        }

        lines
    }

    fn line_highlight(
        line_idx: usize,
        cursor_line: Option<usize>,
        selection: Option<(usize, usize)>,
    ) -> Option<Color> {
        if cursor_line == Some(line_idx) {
            return Some(Color::Rgb(50, 50, 80));
        }
        if let Some((start, end)) = selection {
            if line_idx >= start && line_idx <= end {
                return Some(Color::Rgb(40, 40, 60));
            }
        }
        None
    }

    /// Compute how many visual rows each logical line occupies when wrapped to `width`.
    fn visual_rows_for_line(line: &Line<'_>, width: u16) -> usize {
        if width == 0 {
            return 1;
        }
        let char_count: usize = line.spans.iter().map(|s| s.content.len()).sum();
        if char_count == 0 {
            1
        } else {
            char_count.div_ceil(width as usize)
        }
    }

    /// Convert a logical line index to a visual row offset (sum of wrapped rows for all
    /// preceding logical lines).
    fn logical_to_visual_row(lines: &[Line<'_>], logical: usize, width: u16) -> u16 {
        lines
            .iter()
            .take(logical)
            .map(|l| Self::visual_rows_for_line(l, width) as u16)
            .sum()
    }
}

impl<'a> Widget for DiffViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Account for block borders consuming some of the area
        let inner_width = if let Some(ref block) = self.block {
            block.inner(area).width
        } else {
            area.width
        };

        let lines = match self.diff {
            Some(diff) => Self::build_lines(diff, self.cursor_line, self.selection),
            None => vec![Line::from(Span::styled(
                "No file selected",
                Style::default().fg(Color::DarkGray),
            ))],
        };

        let visual_offset = Self::logical_to_visual_row(&lines, self.scroll_offset, inner_width);

        let mut paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((visual_offset, 0));

        if let Some(block) = self.block {
            paragraph = paragraph.block(block);
        }

        paragraph.render(area, buf);
    }
}
