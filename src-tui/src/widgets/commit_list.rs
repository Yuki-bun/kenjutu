use crate::jj_graph::JjGraphEntry;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

pub struct CommitListWidget<'a> {
    entries: &'a [JjGraphEntry],
    block: Option<Block<'a>>,
}

impl<'a> CommitListWidget<'a> {
    pub fn new(entries: &'a [JjGraphEntry]) -> Self {
        Self {
            entries,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    fn entry_to_item(entry: &JjGraphEntry) -> ListItem<'_> {
        let commit = &entry.commit;
        let mut lines = Vec::with_capacity(1 + entry.continuation_lines.len());

        let gutter_color = if commit.is_working_copy {
            Color::Green
        } else if commit.is_immutable {
            Color::DarkGray
        } else {
            Color::Blue
        };

        let gutter_span = Span::styled(entry.gutter.as_str(), Style::default().fg(gutter_color));

        let change_id_str = commit.change_id.to_string();
        let change_id_short = &change_id_str[..8.min(change_id_str.len())];
        let change_id_span = Span::styled(
            format!("{} ", change_id_short),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

        let summary = Span::styled(commit.summary.as_str(), Style::default().fg(Color::White));

        let author = Span::styled(
            format!("  {}", commit.author),
            Style::default().fg(Color::DarkGray),
        );

        lines.push(Line::from(vec![
            gutter_span,
            change_id_span,
            summary,
            author,
        ]));

        for cont_line in &entry.continuation_lines {
            lines.push(Line::from(Span::styled(
                cont_line.as_str(),
                Style::default().fg(Color::DarkGray),
            )));
        }

        ListItem::new(lines)
    }
}

impl<'a> StatefulWidget for CommitListWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let items: Vec<ListItem> = self.entries.iter().map(Self::entry_to_item).collect();

        let mut list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        if let Some(block) = self.block {
            list = list.block(block);
        }

        StatefulWidget::render(list, area, buf, state);
    }
}
