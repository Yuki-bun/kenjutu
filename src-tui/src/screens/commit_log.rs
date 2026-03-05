use std::path::PathBuf;

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
use crate::jj_log::{self, JjLogOutput, LogCommit};
use crate::jj_ops;
use crate::widgets::commit_list::CommitListWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};
use crate::widgets::text_input::{TextInput, TextInputOutcome};

pub struct CommitLogScreen {
    log: JjLogOutput,
    list_state: ListState,
    local_dir: PathBuf,
    describe_input: Option<TextInput>,
}

impl CommitLogScreen {
    pub fn new(local_dir: PathBuf) -> Self {
        Self {
            log: JjLogOutput {
                lines: Vec::new(),
                commits_by_line: Default::default(),
                commit_lines: Vec::new(),
            },
            list_state: ListState::default(),
            local_dir,
            describe_input: None,
        }
    }

    pub fn load_commits(&mut self) -> Result<()> {
        log::debug!("loading commit log for {}", self.local_dir.display());
        let log = jj_log::get_jj_log(&self.local_dir).context("failed to load commit log")?;

        log::info!(
            "loaded {} lines ({} commits)",
            log.lines.len(),
            log.commit_lines.len()
        );
        self.log = log;
        if self.list_state.selected().is_none() && !self.log.commit_lines.is_empty() {
            // Select the first commit line
            self.list_state.select(Some(self.log.commit_lines[0]));
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ScreenOutcome {
        // If describe input is active, delegate to it
        if let Some(input) = &mut self.describe_input {
            match input.handle_key_event(key) {
                TextInputOutcome::Continue => return ScreenOutcome::Continue,
                TextInputOutcome::Cancel => {
                    self.describe_input = None;
                    return ScreenOutcome::Continue;
                }
                TextInputOutcome::Confirm(message) => {
                    let change_id = self.selected_commit().map(|c| c.change_id);
                    self.describe_input = None;
                    if let Some(change_id) = change_id {
                        if let Err(e) = jj_ops::describe(&self.local_dir, change_id, &message) {
                            return ScreenOutcome::Error(e.to_string());
                        }
                        if let Err(e) = self.load_commits() {
                            return ScreenOutcome::Error(e.to_string());
                        }
                    }
                    return ScreenOutcome::Continue;
                }
            }
        }

        match key.code {
            KeyCode::Char('q') => return ScreenOutcome::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next_commit();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev_commit();
            }
            KeyCode::Char('g') => {
                if let Some(&first) = self.log.commit_lines.first() {
                    self.list_state.select(Some(first));
                }
            }
            KeyCode::Char('G') => {
                if let Some(&last) = self.log.commit_lines.last() {
                    self.list_state.select(Some(last));
                }
            }
            KeyCode::Enter => {
                if let Some(commit) = self.selected_commit().map(LogCommit::to_jj_commit) {
                    return ScreenOutcome::EnterReview(commit);
                }
            }
            KeyCode::Char('r') => {
                if let Err(e) = self.load_commits() {
                    return ScreenOutcome::Error(e.to_string());
                }
            }
            KeyCode::Char('d') => {
                if let Some(commit) = self.selected_commit() {
                    if commit.is_immutable {
                        return ScreenOutcome::Error(
                            "Cannot describe an immutable commit".to_string(),
                        );
                    }
                    let summary = commit.summary.clone();
                    self.describe_input = Some(TextInput::new("Describe: ", &summary));
                }
            }
            KeyCode::Char('n') => {
                if let Some(commit) = self.selected_commit() {
                    let change_id = commit.change_id;
                    if let Err(e) = jj_ops::new_on_top(&self.local_dir, &change_id) {
                        return ScreenOutcome::Error(e.to_string());
                    }
                    if let Err(e) = self.load_commits() {
                        return ScreenOutcome::Error(e.to_string());
                    }
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
                self.local_dir.to_string_lossy(),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(header, chunks[0]);

        let block = Block::default().borders(Borders::NONE);
        let widget = CommitListWidget::new(&self.log).block(block);
        frame.render_stateful_widget(widget, chunks[1], &mut self.list_state);

        if let Some(input) = &self.describe_input {
            frame.render_widget(input.widget(), chunks[2]);
        } else {
            let bindings = [
                Binding::new("j/k", "navigate"),
                Binding::new("Enter", "review"),
                Binding::new("d", "describe"),
                Binding::new("n", "new"),
                Binding::new("r", "refresh"),
                Binding::new("g/G", "top/bottom"),
                Binding::new("q", "quit"),
            ];
            let status = StatusBarWidget::new(&bindings);
            frame.render_widget(status, chunks[2]);
        }
    }

    fn selected_commit(&self) -> Option<&LogCommit> {
        self.list_state
            .selected()
            .and_then(|i| self.log.commits_by_line.get(&i))
    }

    /// Move selection to the next commit line (skip non-commit lines).
    fn select_next_commit(&mut self) {
        let current = self.list_state.selected().unwrap_or(0);
        // Find next commit line after current
        if let Some(&next) = self.log.commit_lines.iter().find(|&&l| l > current) {
            self.list_state.select(Some(next));
        }
    }

    /// Move selection to the previous commit line (skip non-commit lines).
    fn select_prev_commit(&mut self) {
        let current = self.list_state.selected().unwrap_or(0);
        // Find previous commit line before current
        if let Some(&prev) = self.log.commit_lines.iter().rfind(|&&l| l < current) {
            self.list_state.select(Some(prev));
        }
    }
}
