use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListState},
    Frame,
};

use super::ScreenOutcome;
use crate::jj_graph::{self, GraphCommit, JjGraphEntry};
use crate::widgets::commit_list::CommitListWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};

pub struct CommitLogScreen {
    entries: Vec<JjGraphEntry>,
    list_state: ListState,
    local_dir: String,
}

impl CommitLogScreen {
    pub fn new(local_dir: String) -> Self {
        Self {
            entries: Vec::new(),
            list_state: ListState::default(),
            local_dir,
        }
    }

    pub fn load_commits(&mut self) -> Result<()> {
        log::debug!("loading commit log for {}", self.local_dir);
        let entries = jj_graph::get_log_with_graph(&self.local_dir)
            .context("failed to load commit log")?;

        log::info!("loaded {} commits", entries.len());
        self.entries = entries;
        if self.list_state.selected().is_none() && !self.entries.is_empty() {
            self.list_state.select(Some(0));
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ScreenOutcome {
        match key.code {
            KeyCode::Char('q') => return ScreenOutcome::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.list_state.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.list_state.select_previous();
            }
            KeyCode::Char('g') => {
                self.list_state.select_first();
            }
            KeyCode::Char('G') => {
                self.list_state.select_last();
            }
            KeyCode::Enter => {
                if let Some(commit) = self.selected_commit().cloned() {
                    return ScreenOutcome::EnterReview(commit);
                }
            }
            KeyCode::Char('r') => {
                if let Err(e) = self.load_commits() {
                    return ScreenOutcome::Error(e.to_string());
                }
            }
            _ => {}
        }
        ScreenOutcome::Continue
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

        let header = Line::from(vec![
            Span::styled(
                " Commit Log: ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                self.local_dir.as_str(),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(header, chunks[0]);

        let block = Block::default().borders(Borders::NONE);
        let widget = CommitListWidget::new(&self.entries).block(block);
        frame.render_stateful_widget(widget, chunks[1], &mut self.list_state);

        let bindings = [
            Binding::new("j/k", "navigate"),
            Binding::new("Enter", "review"),
            Binding::new("r", "refresh"),
            Binding::new("g/G", "top/bottom"),
            Binding::new("q", "quit"),
        ];
        let status = StatusBarWidget::new(&bindings);
        frame.render_widget(status, chunks[2]);
    }

    fn selected_commit(&self) -> Option<&GraphCommit> {
        self.list_state
            .selected()
            .and_then(|i| self.entries.get(i))
            .map(|entry| &entry.commit)
    }
}
