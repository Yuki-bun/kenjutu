use kenjutu_core::models::{CommitGraph, CommitRow, ElisionRow, GraphRow};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

pub struct CommitListWidget<'a> {
    graph: &'a CommitGraph,
    block: Option<Block<'a>>,
}

impl<'a> CommitListWidget<'a> {
    pub fn new(graph: &'a CommitGraph) -> Self {
        Self { graph, block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }
}

/// Build a gutter string from structured column data.
///
/// Each column occupies 2 characters: the symbol and a trailing space.
/// Pass-through columns get `│`, the node column gets the node character,
/// and all other positions are spaces.
fn build_gutter(
    node_col: usize,
    node_char: char,
    passing_columns: &[usize],
    max_columns: usize,
) -> String {
    let width = max_columns * 2;
    let mut gutter = vec![' '; width];

    for &col in passing_columns {
        if col < max_columns {
            gutter[col * 2] = '│';
        }
    }

    // Node character overwrites any pass-through at the same column
    if node_col < max_columns {
        gutter[node_col * 2] = node_char;
    }

    // Trim trailing spaces but keep at least one space after the last symbol
    let last_non_space = gutter.iter().rposition(|&c| c != ' ').unwrap_or(0);
    gutter.truncate(last_non_space + 2);

    gutter.into_iter().collect()
}

fn commit_row_to_item(row: &CommitRow, max_columns: usize) -> ListItem<'_> {
    let commit = &row.commit;

    let node_char = if commit.is_working_copy {
        '@'
    } else if commit.is_immutable {
        '◆'
    } else {
        '○'
    };

    let gutter_color = if commit.is_working_copy {
        Color::Green
    } else if commit.is_immutable {
        Color::DarkGray
    } else {
        Color::Blue
    };

    let gutter = build_gutter(row.column, node_char, &row.passing_columns, max_columns);
    let gutter_span = Span::styled(gutter, Style::default().fg(gutter_color));

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

    ListItem::new(Line::from(vec![
        gutter_span,
        change_id_span,
        summary,
        author,
    ]))
}

fn elision_row_to_item(row: &ElisionRow, max_columns: usize) -> ListItem<'static> {
    let gutter = build_gutter(row.column, '~', &row.passing_columns, max_columns);
    let gutter_span = Span::styled(gutter, Style::default().fg(Color::DarkGray));

    let label = Span::styled("(elided revisions)", Style::default().fg(Color::DarkGray));

    ListItem::new(Line::from(vec![gutter_span, label]))
}

impl<'a> StatefulWidget for CommitListWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let max_columns = self.graph.max_columns;
        let items: Vec<ListItem> = self
            .graph
            .rows
            .iter()
            .map(|row| match row {
                GraphRow::Commit(commit_row) => commit_row_to_item(commit_row, max_columns),
                GraphRow::Elision(elision_row) => elision_row_to_item(elision_row, max_columns),
            })
            .collect();

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
