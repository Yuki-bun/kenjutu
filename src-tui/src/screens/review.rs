use anyhow::{Context, Result};
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use git2::Repository;
use kenjutu_core::models::{FileChangeStatus, FileEntry, ReviewStatus};

use crate::jj_ops;
use kenjutu_core::models::JjCommit;
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, ListState},
    Frame,
};

use super::ScreenOutcome;
use crate::widgets::file_list::FileListWidget;
use crate::widgets::header::HeaderWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};
use crate::widgets::text_input::{TextInput, TextInputOutcome};

mod diff_panel;
mod diff_view;

use diff_view::{DiffView, DiffViewOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFocus {
    FileList,
    DiffView,
}

pub struct ReviewScreen {
    commit: JjCommit,
    change_id: ChangeId,
    commit_id: CommitId,

    files: Vec<FileEntry>,
    file_selected_index: usize,

    diff_view: DiffView,

    focus: ReviewFocus,
    file_list_state: ListState,

    local_dir: String,
    describe_input: Option<TextInput>,
}

impl ReviewScreen {
    pub fn new(
        commit: JjCommit,
        commit_id: CommitId,
        repository: &Repository,
        local_dir: String,
    ) -> Result<Self> {
        let (change_id, files) =
            diff::generate_file_list(repository, commit_id).context("failed to load file list")?;

        log::info!("loaded {} files for review", files.len());

        let mut screen = Self {
            commit,
            change_id,
            commit_id,
            files,
            file_selected_index: 0,
            diff_view: DiffView::new(change_id, commit_id),
            focus: ReviewFocus::FileList,
            file_list_state: ListState::default(),
            local_dir,
            describe_input: None,
        };
        screen.file_list_state.select(Some(0));
        screen.load_current_file_diff(repository);
        Ok(screen)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, repository: &Repository) -> ScreenOutcome {
        // If describe input is active, delegate to it
        if let Some(input) = &mut self.describe_input {
            match input.handle_key_event(key) {
                TextInputOutcome::Continue => return ScreenOutcome::Continue,
                TextInputOutcome::Cancel => {
                    self.describe_input = None;
                    return ScreenOutcome::Continue;
                }
                TextInputOutcome::Confirm(message) => {
                    let change_id = self.change_id;
                    self.describe_input = None;
                    if let Err(e) = jj_ops::describe(&self.local_dir, &change_id, &message) {
                        return ScreenOutcome::Error(e.to_string());
                    }
                    self.commit.summary = message;
                    return ScreenOutcome::Continue;
                }
            }
        }

        match self.focus {
            ReviewFocus::FileList => self.handle_file_list_key(key, repository),
            ReviewFocus::DiffView => {
                match self.diff_view.handle_key_event(key, repository) {
                    DiffViewOutcome::Continue => {}
                    DiffViewOutcome::ExitToFileList => self.focus = ReviewFocus::FileList,
                    DiffViewOutcome::NextFile => {
                        if let Some(e) = self.select_next_file_and_load(repository) {
                            return ScreenOutcome::Error(e);
                        }
                    }
                    DiffViewOutcome::PrevFile => {
                        if let Some(e) = self.select_prev_file_and_load(repository) {
                            return ScreenOutcome::Error(e);
                        }
                    }
                    DiffViewOutcome::ActionApplied => {
                        if let Err(e) = self.reload_file_list(repository) {
                            return ScreenOutcome::Error(e.to_string());
                        }
                        self.load_current_file_diff(repository);
                    }
                    DiffViewOutcome::Error(e) => return ScreenOutcome::Error(e),
                }
                ScreenOutcome::Continue
            }
        }
    }

    fn handle_file_list_key(&mut self, key: KeyEvent, repository: &Repository) -> ScreenOutcome {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return ScreenOutcome::ExitReview,
            KeyCode::Tab | KeyCode::Enter => {
                self.focus = ReviewFocus::DiffView;
            }
            KeyCode::BackTab => {
                self.focus = ReviewFocus::DiffView;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(e) = self.select_next_file_and_load(repository) {
                    return ScreenOutcome::Error(e);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(e) = self.select_prev_file_and_load(repository) {
                    return ScreenOutcome::Error(e);
                }
            }
            KeyCode::Char(' ') => {
                if let Err(e) = self.toggle_file_reviewed(repository) {
                    return ScreenOutcome::Error(e.to_string());
                }
            }
            KeyCode::Char('r') => {
                if let Err(e) = self.reload_file_list(repository) {
                    return ScreenOutcome::Error(e.to_string());
                }
                self.load_current_file_diff(repository);
            }
            KeyCode::Char('d') => {
                if self.commit.is_immutable {
                    return ScreenOutcome::Error("Cannot describe an immutable commit".to_string());
                }
                let summary = self.commit.summary.clone();
                self.describe_input = Some(TextInput::new("Describe: ", &summary));
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

        // Header
        let change_id_str = self.commit.change_id.to_string();
        let header = HeaderWidget::new("Review", &change_id_str, &self.commit.summary);
        frame.render_widget(header, chunks[0]);

        // Main content: file list | diff view(s)
        let main_chunks =
            Layout::horizontal([Constraint::Length(60), Constraint::Fill(1)]).split(chunks[1]);

        // File list
        let file_list_focused = self.focus == ReviewFocus::FileList;
        let file_block_style = if file_list_focused {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let reviewed_count = self
            .files
            .iter()
            .filter(|f| f.review_status == ReviewStatus::Reviewed)
            .count();
        let file_block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(file_block_style)
            .title(format!(" Files {}/{} ", reviewed_count, self.files.len()));
        let file_widget = FileListWidget::new(&self.files, file_list_focused).block(file_block);
        frame.render_stateful_widget(file_widget, main_chunks[0], &mut self.file_list_state);

        // Diff view
        let diff_focused = self.focus == ReviewFocus::DiffView;
        let file_title: String = self
            .selected_file()
            .and_then(|f| f.new_path.as_deref().or(f.old_path.as_deref()))
            .unwrap_or("")
            .to_string();
        self.diff_view
            .render(frame, main_chunks[1], diff_focused, &file_title);

        // Status bar
        if let Some(input) = &self.describe_input {
            frame.render_widget(input.widget(), chunks[2]);
        } else {
            let bindings = match self.focus {
                ReviewFocus::FileList => vec![
                    Binding::new("j/k", "navigate"),
                    Binding::new("Enter/Tab", "diff view"),
                    Binding::new("Space", "mark reviewed"),
                    Binding::new("d", "describe"),
                    Binding::new("Esc/q", "back"),
                ],
                ReviewFocus::DiffView => {
                    let action_label = self.diff_view.action_label();
                    let mut b = vec![
                        Binding::new("j/k", "navigate"),
                        Binding::new("C-d/C-u", "page"),
                        Binding::new("v", "select"),
                        Binding::new("Space", action_label),
                    ];
                    if self.diff_view.is_split() {
                        b.push(Binding::new("Tab/S-Tab", "switch panel"));
                    }
                    b.push(Binding::new("n/N", "next/prev file"));
                    b.push(Binding::new("Esc/q", "back"));
                    b
                }
            };
            let status = StatusBarWidget::new(&bindings);
            frame.render_widget(status, chunks[2]);
        }
    }

    fn select_next_file(&mut self) {
        if !self.files.is_empty() {
            self.file_selected_index = (self.file_selected_index + 1).min(self.files.len() - 1);
        }
    }

    fn select_prev_file(&mut self) {
        self.file_selected_index = self.file_selected_index.saturating_sub(1);
    }

    fn selected_file(&self) -> Option<&FileEntry> {
        self.files.get(self.file_selected_index)
    }

    fn load_current_file_diff(&mut self, repository: &Repository) {
        let Some(file) = self.files.get(self.file_selected_index) else {
            self.diff_view.clear();
            return;
        };

        if file.is_binary {
            self.diff_view.clear();
            return;
        }

        let (file_path, old_path) = resolve_file_paths(file);
        self.diff_view
            .load(repository, &file_path, old_path.as_deref());
    }

    fn select_next_file_and_load(&mut self, repository: &Repository) -> Option<String> {
        let old_idx = self.file_selected_index;
        self.select_next_file();
        if self.file_selected_index != old_idx {
            self.file_list_state.select(Some(self.file_selected_index));
            self.load_current_file_diff(repository);
        }
        None
    }

    fn select_prev_file_and_load(&mut self, repository: &Repository) -> Option<String> {
        let old_idx = self.file_selected_index;
        self.select_prev_file();
        if self.file_selected_index != old_idx {
            self.file_list_state.select(Some(self.file_selected_index));
            self.load_current_file_diff(repository);
        }
        None
    }

    fn toggle_file_reviewed(&mut self, repository: &Repository) -> Result<()> {
        let Some(file) = self.files.get(self.file_selected_index) else {
            log::warn!(
                "toggle_file_reviewed: no file at index {}",
                self.file_selected_index
            );
            return Ok(());
        };

        let file_path_display = file
            .new_path
            .as_deref()
            .or(file.old_path.as_deref())
            .unwrap_or("<unknown>");
        let is_reviewed = file.review_status == ReviewStatus::Reviewed;
        log::info!(
            "toggle_file_reviewed: file={} currently_reviewed={} change_id={} commit_id={}",
            file_path_display,
            is_reviewed,
            self.change_id,
            self.commit_id
        );

        let (file_path, old_path) = resolve_file_paths(file);

        // Scoped block so the MarkerCommit (and its exclusive file lock)
        // is dropped before reload_file_list tries to read markers.
        {
            log::debug!("opening marker commit");
            let mut marker =
                marker_commit::MarkerCommit::get(repository, self.change_id, self.commit_id)
                    .context("Failed to open marker commit")?;

            if is_reviewed {
                log::debug!("unmarking file as reviewed: {:?}", file_path);
                marker
                    .unmark_file_reviewed(&file_path, old_path.as_deref())
                    .context("Failed to unmark file")?;
            } else {
                log::debug!("marking file as reviewed: {:?}", file_path);
                marker
                    .mark_file_reviewed(&file_path, old_path.as_deref())
                    .context("Failed to mark file")?;
            }

            log::debug!("writing marker commit");
            let marker_id = marker.write().context("Failed to write marker commit")?;
            log::info!("marker commit written: {}", marker_id);
        }

        self.reload_file_list(repository)
            .context("Failed to reload file list")?;
        self.load_current_file_diff(repository);
        Ok(())
    }

    fn reload_file_list(&mut self, repository: &Repository) -> Result<()> {
        let (_change_id, files) = diff::generate_file_list(repository, self.commit_id)
            .context("failed to reload file list")?;
        self.files = files;
        if self.file_selected_index >= self.files.len() && !self.files.is_empty() {
            self.file_selected_index = self.files.len() - 1;
        }
        self.file_list_state.select(Some(self.file_selected_index));
        Ok(())
    }
}

fn resolve_file_paths(file: &kenjutu_core::models::FileEntry) -> (PathBuf, Option<PathBuf>) {
    match file.status {
        FileChangeStatus::Deleted => {
            let path = file.old_path.as_deref().unwrap_or("");
            (PathBuf::from(path), None)
        }
        FileChangeStatus::Renamed => {
            let new = file.new_path.as_deref().unwrap_or("");
            let old = file.old_path.as_deref().map(PathBuf::from);
            (PathBuf::from(new), old)
        }
        _ => {
            let path = file.new_path.as_deref().unwrap_or("");
            (PathBuf::from(path), None)
        }
    }
}
