use kenjutu_core::models::JjCommit;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

pub struct CommitListWidget<'a> {
    commits: &'a [JjCommit],
    block: Option<Block<'a>>,
}

impl<'a> CommitListWidget<'a> {
    pub fn new(commits: &'a [JjCommit]) -> Self {
        Self {
            commits,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    fn commit_to_item(commit: &JjCommit) -> ListItem<'_> {
        let indicator = if commit.is_working_copy {
            Span::styled(
                "@ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else if commit.is_immutable {
            Span::styled("◆ ", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled("○ ", Style::default().fg(Color::Blue))
        };

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

        let line = Line::from(vec![indicator, change_id_span, summary, author]);
        ListItem::new(line)
    }
}

impl<'a> StatefulWidget for CommitListWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let items: Vec<ListItem> = self.commits.iter().map(Self::commit_to_item).collect();

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
