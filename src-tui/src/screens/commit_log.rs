use anyhow::{Context, Result};
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
    list_state: ListState,
    local_dir: String,
}

impl CommitLogScreen {
    pub fn new(local_dir: String) -> Self {
        Self {
            commits: Vec::new(),
            list_state: ListState::default(),
            local_dir,
        }
    }

    pub fn load_commits(&mut self) -> Result<()> {
        log::debug!("loading commit log for {}", self.local_dir);
        let result = jj::get_log(&self.local_dir).context("failed to load commit log")?;

        log::info!("loaded {} commits", result.commits.len());
        self.commits = result.commits;
        if self.list_state.selected().is_none() && !self.commits.is_empty() {
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

    fn selected_commit(&self) -> Option<&JjCommit> {
        self.list_state.selected().and_then(|i| self.commits.get(i))
    }
}
