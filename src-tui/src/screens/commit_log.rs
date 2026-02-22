use crossterm::event::{KeyCode, KeyEvent};
use kenjutu_core::models::JjCommit;
use kenjutu_core::services::jj;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListState},
    Frame,
};

use super::ScreenOutcome;
use crate::widgets::commit_list::CommitListWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};

pub struct CommitLogScreen {
    commits: Vec<JjCommit>,
    selected_index: usize,
    list_state: ListState,
    local_dir: String,
}

impl CommitLogScreen {
    pub fn new(local_dir: String) -> Self {
        Self {
            commits: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            local_dir,
        }
    }

    pub fn load_commits(&mut self) -> Result<(), String> {
        log::debug!("loading commit log for {}", self.local_dir);
        match jj::get_log(&self.local_dir) {
            Ok(result) => {
                log::info!("loaded {} commits", result.commits.len());
                self.commits = result.commits;
                self.selected_index = 0;
                self.list_state.select(Some(0));
                Ok(())
            }
            Err(e) => {
                log::error!("failed to load jj log: {}", e);
                Err(format!("Failed to load jj log: {}", e))
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ScreenOutcome {
        match key.code {
            KeyCode::Char('q') => return ScreenOutcome::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
                self.list_state.select(Some(self.selected_index));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev();
                self.list_state.select(Some(self.selected_index));
            }
            KeyCode::Char('g') => {
                self.select_first();
                self.list_state.select(Some(self.selected_index));
            }
            KeyCode::Char('G') => {
                self.select_last();
                self.list_state.select(Some(self.selected_index));
            }
            KeyCode::Enter => {
                if let Some(commit) = self.selected_commit().cloned() {
                    return ScreenOutcome::EnterReview(commit);
                }
            }
            KeyCode::Char('r') => {
                if let Err(e) = self.load_commits() {
                    return ScreenOutcome::Error(e);
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
        let widget = CommitListWidget::new(&self.commits).block(block);
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

    fn select_next(&mut self) {
        if !self.commits.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.commits.len() - 1);
        }
    }

    fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    fn select_first(&mut self) {
        self.selected_index = 0;
    }

    fn select_last(&mut self) {
        if !self.commits.is_empty() {
            self.selected_index = self.commits.len() - 1;
        }
    }

    fn selected_commit(&self) -> Option<&JjCommit> {
        self.commits.get(self.selected_index)
    }
}
