use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git2::Repository;
use kenjutu_core::models::{FileChangeStatus, FileDiff, FileEntry, JjCommit, ReviewStatus};
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, ListState},
    Frame,
};

use super::ScreenOutcome;
use crate::widgets::diff_view::DiffViewWidget;
use crate::widgets::file_list::FileListWidget;
use crate::widgets::header::HeaderWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFocus {
    FileList,
    DiffView,
}

pub struct ReviewScreen {
    pub commit: JjCommit,
    pub change_id: ChangeId,
    pub commit_id: CommitId,

    pub files: Vec<FileEntry>,
    pub file_selected_index: usize,

    pub current_diff: Option<FileDiff>,
    pub diff_scroll_offset: usize,
    pub diff_total_lines: usize,

    pub focus: ReviewFocus,
    file_list_state: ListState,
}

impl ReviewScreen {
    pub fn new(
        commit: JjCommit,
        commit_id: CommitId,
        repository: &Repository,
    ) -> Result<Self, String> {
        let (change_id, files) = diff::generate_file_list(repository, commit_id)
            .map_err(|e| format!("Failed to load file list: {}", e))?;

        log::info!("loaded {} files for review", files.len());

        let mut screen = Self {
            commit,
            change_id,
            commit_id,
            files,
            file_selected_index: 0,
            current_diff: None,
            diff_scroll_offset: 0,
            diff_total_lines: 0,
            focus: ReviewFocus::FileList,
            file_list_state: ListState::default(),
        };
        screen.file_list_state.select(Some(0));
        screen.load_current_file_diff(repository);
        Ok(screen)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, repository: &Repository) -> ScreenOutcome {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return ScreenOutcome::ExitReview,
            KeyCode::Tab => {
                self.focus = match self.focus {
                    ReviewFocus::FileList => ReviewFocus::DiffView,
                    ReviewFocus::DiffView => ReviewFocus::FileList,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                ReviewFocus::FileList => {
                    if let Some(e) = self.select_next_file_and_load(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffView => {
                    self.scroll_diff_down(1);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                ReviewFocus::FileList => {
                    if let Some(e) = self.select_prev_file_and_load(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffView => {
                    self.scroll_diff_up(1);
                }
            },
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_diff_down(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_diff_up(20);
            }
            KeyCode::Char('g') => {
                self.diff_scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                self.diff_scroll_offset = self.diff_total_lines.saturating_sub(1);
            }
            KeyCode::Enter => {
                if self.focus == ReviewFocus::FileList {
                    self.focus = ReviewFocus::DiffView;
                }
            }
            KeyCode::Char('n') => {
                if let Some(e) = self.select_next_file_and_load(repository) {
                    return ScreenOutcome::Error(e);
                }
            }
            KeyCode::Char('N') => {
                if let Some(e) = self.select_prev_file_and_load(repository) {
                    return ScreenOutcome::Error(e);
                }
            }
            KeyCode::Char(' ') => {
                if let Err(e) = self.toggle_file_reviewed(repository) {
                    return ScreenOutcome::Error(e);
                }
            }
            KeyCode::Char('R') => {
                if let Err(e) = self.reload_file_list(repository) {
                    return ScreenOutcome::Error(e);
                }
                self.load_current_file_diff(repository);
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

        // Main content: file list | diff view
        let main_chunks =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[1]);

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
        let diff_block_style = if diff_focused {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let diff_title = self
            .selected_file()
            .and_then(|f| f.new_path.as_deref().or(f.old_path.as_deref()))
            .unwrap_or("");

        let diff_block = Block::default()
            .borders(Borders::NONE)
            .border_style(diff_block_style)
            .title(format!(" {} ", diff_title));

        let scroll_offset = self.diff_scroll_offset;
        let diff_widget =
            DiffViewWidget::new(self.current_diff.as_ref(), scroll_offset).block(diff_block);
        frame.render_widget(diff_widget, main_chunks[1]);

        // Status bar
        let bindings = match self.focus {
            ReviewFocus::FileList => vec![
                Binding::new("j/k", "navigate"),
                Binding::new("Enter/Tab", "diff view"),
                Binding::new("Space", "mark reviewed"),
                Binding::new("Esc/q", "back"),
            ],
            ReviewFocus::DiffView => vec![
                Binding::new("j/k", "scroll"),
                Binding::new("C-d/C-u", "page scroll"),
                Binding::new("Tab", "file list"),
                Binding::new("n/N", "next/prev file"),
                Binding::new("Esc/q", "back"),
            ],
        };
        let status = StatusBarWidget::new(&bindings);
        frame.render_widget(status, chunks[2]);
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

    fn scroll_diff_down(&mut self, amount: usize) {
        if self.diff_total_lines > 0 {
            self.diff_scroll_offset =
                (self.diff_scroll_offset + amount).min(self.diff_total_lines.saturating_sub(1));
        }
    }

    fn scroll_diff_up(&mut self, amount: usize) {
        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(amount);
    }

    fn load_current_file_diff(&mut self, repository: &Repository) {
        let Some(file) = self.files.get(self.file_selected_index) else {
            self.current_diff = None;
            return;
        };

        if file.is_binary {
            self.current_diff = None;
            self.diff_total_lines = 0;
            self.diff_scroll_offset = 0;
            return;
        }

        let (file_path, old_path) = resolve_file_paths(file);

        match diff::generate_single_file_diff(
            repository,
            self.commit_id,
            &file_path,
            old_path.as_deref(),
        ) {
            Ok(diff) => {
                self.diff_total_lines = diff.hunks.iter().map(|h| h.lines.len() + 1).sum();
                self.current_diff = Some(diff);
                self.diff_scroll_offset = 0;
            }
            Err(e) => {
                log::error!("failed to load diff: {}", e);
                self.current_diff = None;
            }
        }
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

    fn toggle_file_reviewed(&mut self, repository: &Repository) -> Result<(), String> {
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
                    .map_err(|e| format!("Failed to open marker commit: {}", e))?;

            if is_reviewed {
                log::debug!("unmarking file as reviewed: {:?}", file_path);
                marker
                    .unmark_file_reviewed(&file_path, old_path.as_deref())
                    .map_err(|e| format!("Failed to unmark: {}", e))?;
            } else {
                log::debug!("marking file as reviewed: {:?}", file_path);
                marker
                    .mark_file_reviewed(&file_path, old_path.as_deref())
                    .map_err(|e| format!("Failed to mark: {}", e))?;
            }

            log::debug!("writing marker commit");
            let marker_id = marker
                .write()
                .map_err(|e| format!("Failed to write: {}", e))?;
            log::info!("marker commit written: {}", marker_id);
        }

        self.reload_file_list(repository)?;
        Ok(())
    }

    fn reload_file_list(&mut self, repository: &Repository) -> Result<(), String> {
        match diff::generate_file_list(repository, self.commit_id) {
            Ok((_change_id, files)) => {
                self.files = files;
                if self.file_selected_index >= self.files.len() && !self.files.is_empty() {
                    self.file_selected_index = self.files.len() - 1;
                }
                self.file_list_state.select(Some(self.file_selected_index));
                Ok(())
            }
            Err(e) => Err(format!("Failed to reload: {}", e)),
        }
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
