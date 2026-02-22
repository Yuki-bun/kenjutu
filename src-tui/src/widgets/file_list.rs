use kenjutu_core::models::{FileChangeStatus, FileEntry, ReviewStatus};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

pub struct FileListWidget<'a> {
    files: &'a [FileEntry],
    block: Option<Block<'a>>,
    is_focused: bool,
}

impl<'a> FileListWidget<'a> {
    pub fn new(files: &'a [FileEntry], is_focused: bool) -> Self {
        Self {
            files,
            block: None,
            is_focused,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    fn file_to_item(file: &FileEntry) -> ListItem<'_> {
        let review_indicator = match file.review_status {
            ReviewStatus::Reviewed => Span::styled("✓ ", Style::default().fg(Color::Green)),
            ReviewStatus::PartiallyReviewed => {
                Span::styled("● ", Style::default().fg(Color::Yellow))
            }
            ReviewStatus::Unreviewed => Span::raw("  "),
            ReviewStatus::ReviewedReverted => {
                Span::styled("⟲ ", Style::default().fg(Color::DarkGray))
            }
        };

        let path = file
            .new_path
            .as_deref()
            .or(file.old_path.as_deref())
            .unwrap_or("<unknown>");

        let path_span = Span::styled(path, Style::default().fg(Color::White));

        let (status_char, status_color) = match file.status {
            FileChangeStatus::Added => ("A", Color::Green),
            FileChangeStatus::Modified => ("M", Color::Yellow),
            FileChangeStatus::Deleted => ("D", Color::Red),
            FileChangeStatus::Renamed => ("R", Color::Blue),
            FileChangeStatus::Copied => ("C", Color::Cyan),
            FileChangeStatus::Typechange => ("T", Color::Magenta),
        };

        let status_span = Span::styled(
            format!(" {}", status_char),
            Style::default().fg(status_color),
        );

        let stats = if file.additions > 0 || file.deletions > 0 {
            Span::styled(
                format!(" +{} -{}", file.additions, file.deletions),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw("")
        };

        let line = Line::from(vec![review_indicator, path_span, status_span, stats]);
        ListItem::new(line)
    }
}

impl<'a> StatefulWidget for FileListWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let items: Vec<ListItem> = self.files.iter().map(Self::file_to_item).collect();

        let highlight_color = if self.is_focused {
            Color::DarkGray
        } else {
            Color::Rgb(40, 40, 40)
        };

        let mut list = List::new(items).highlight_style(
            Style::default()
                .bg(highlight_color)
                .add_modifier(Modifier::BOLD),
        );

        if let Some(block) = self.block {
            list = list.block(block);
        }

        StatefulWidget::render(list, area, buf, state);
    }
}
