use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

use crate::jj_log::JjLogOutput;

pub struct CommitListWidget<'a> {
    log: &'a JjLogOutput,
    block: Option<Block<'a>>,
}

impl<'a> CommitListWidget<'a> {
    pub fn new(log: &'a JjLogOutput) -> Self {
        Self { log, block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }
}

impl<'a> StatefulWidget for CommitListWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let items: Vec<ListItem> = self
            .log
            .lines
            .iter()
            .map(|line| ListItem::new(line.clone()))
            .collect();

        let mut list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));

        if let Some(block) = self.block {
            list = list.block(block);
        }

        StatefulWidget::render(list, area, buf, state);
    }
}
