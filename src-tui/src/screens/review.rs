use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git2::Repository;
use kenjutu_core::models::{FileChangeStatus, FileEntry, JjCommit, ReviewStatus};
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, ListState},
    Frame,
};

use super::diff_panel::DiffPanelState;
use super::ScreenOutcome;
use crate::widgets::diff_view::DiffViewWidget;
use crate::widgets::file_list::FileListWidget;
use crate::widgets::header::HeaderWidget;
use crate::widgets::status_bar::{Binding, StatusBarWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFocus {
    FileList,
    DiffView,
    DiffLeft,
    DiffRight,
}

pub struct ReviewScreen {
    pub commit: JjCommit,
    pub change_id: ChangeId,
    pub commit_id: CommitId,

    pub files: Vec<FileEntry>,
    pub file_selected_index: usize,

    /// B→T diff (single panel when not split)
    pub main_panel: DiffPanelState,
    /// M→T diff (left panel when split)
    pub remaining_panel: DiffPanelState,
    /// B→M diff (right panel when split)
    pub reviewed_panel: DiffPanelState,

    diff_view_height: u16,

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
            main_panel: DiffPanelState::new(),
            remaining_panel: DiffPanelState::new(),
            reviewed_panel: DiffPanelState::new(),
            diff_view_height: 0,
            focus: ReviewFocus::FileList,
            file_list_state: ListState::default(),
        };
        screen.file_list_state.select(Some(0));
        screen.load_current_file_diff(repository);
        Ok(screen)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, repository: &Repository) -> ScreenOutcome {
        let vh = self.diff_view_height;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => match self.focus {
                ReviewFocus::FileList => return ScreenOutcome::ExitReview,
                ReviewFocus::DiffView | ReviewFocus::DiffLeft | ReviewFocus::DiffRight => {
                    if self.active_panel().selection_active {
                        self.active_panel_mut().cancel_selection();
                    } else {
                        self.focus = ReviewFocus::FileList;
                    }
                }
            },
            KeyCode::Tab => {
                self.focus = match self.focus {
                    ReviewFocus::FileList => self.default_diff_focus(),
                    ReviewFocus::DiffView | ReviewFocus::DiffRight => ReviewFocus::FileList,
                    ReviewFocus::DiffLeft => ReviewFocus::DiffRight,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                ReviewFocus::FileList => {
                    if let Some(e) = self.select_next_file_and_load(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffView | ReviewFocus::DiffLeft | ReviewFocus::DiffRight => {
                    self.active_panel_mut().move_cursor(1, vh);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                ReviewFocus::FileList => {
                    if let Some(e) = self.select_prev_file_and_load(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffView | ReviewFocus::DiffLeft | ReviewFocus::DiffRight => {
                    self.active_panel_mut().move_cursor(-1, vh);
                }
            },
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half = (vh as i32 / 2).max(1);
                self.active_panel_mut().move_cursor(half, vh);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half = (vh as i32 / 2).max(1);
                self.active_panel_mut().move_cursor(-half, vh);
            }
            KeyCode::Char('g') => {
                self.active_panel_mut().set_cursor(0, vh);
            }
            KeyCode::Char('G') => {
                let max = self.active_panel().total_lines.saturating_sub(1);
                self.active_panel_mut().set_cursor(max, vh);
            }
            KeyCode::Enter => {
                if self.focus == ReviewFocus::FileList {
                    self.focus = self.default_diff_focus();
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
            KeyCode::Char('v') => match self.focus {
                ReviewFocus::DiffView | ReviewFocus::DiffLeft | ReviewFocus::DiffRight => {
                    self.active_panel_mut().toggle_selection();
                }
                _ => {}
            },
            KeyCode::Char(' ') => match self.focus {
                ReviewFocus::FileList => {
                    if let Err(e) = self.toggle_file_reviewed(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffView | ReviewFocus::DiffLeft => {
                    if let Err(e) = self.mark_lines_reviewed(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
                ReviewFocus::DiffRight => {
                    if let Err(e) = self.unmark_lines_reviewed(repository) {
                        return ScreenOutcome::Error(e);
                    }
                }
            },
            KeyCode::Char('r') => {
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

        // Main content: file list | diff view(s)
        let is_split = self.remaining_panel.diff.is_some();
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

        if is_split {
            // Split diff view: M→T (left) | B→T (right)
            let diff_chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(main_chunks[1]);

            self.diff_view_height = diff_chunks[0].height;

            // Left panel: M→T (remaining)
            let left_focused = self.focus == ReviewFocus::DiffLeft;
            let left_block_style = if left_focused {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let left_block = Block::default()
                .borders(Borders::RIGHT)
                .border_style(left_block_style)
                .title(" Remaining (M\u{2192}T) ");

            let left_cursor = if left_focused {
                Some(self.remaining_panel.cursor_line)
            } else {
                None
            };
            let left_selection = self.remaining_panel.selection_range();
            let mut left_widget = DiffViewWidget::new(
                self.remaining_panel.diff.as_ref(),
                self.remaining_panel.scroll_offset,
            )
            .block(left_block);
            if let Some(c) = left_cursor {
                left_widget = left_widget.cursor_line(c);
            }
            if let Some(s) = left_selection {
                left_widget = left_widget.selection(s);
            }
            frame.render_widget(left_widget, diff_chunks[0]);

            // Right panel: B→M (reviewed)
            let right_focused = self.focus == ReviewFocus::DiffRight;
            let right_block_style = if right_focused {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let right_block = Block::default()
                .borders(Borders::NONE)
                .border_style(right_block_style)
                .title(" Reviewed (B\u{2192}M) ");

            let right_cursor = if right_focused {
                Some(self.reviewed_panel.cursor_line)
            } else {
                None
            };
            let right_selection = self.reviewed_panel.selection_range();
            let mut right_widget = DiffViewWidget::new(
                self.reviewed_panel.diff.as_ref(),
                self.reviewed_panel.scroll_offset,
            )
            .block(right_block);
            if let Some(c) = right_cursor {
                right_widget = right_widget.cursor_line(c);
            }
            if let Some(s) = right_selection {
                right_widget = right_widget.selection(s);
            }
            frame.render_widget(right_widget, diff_chunks[1]);
        } else {
            // Single diff view: B→T (which equals M→T when not partial)
            self.diff_view_height = main_chunks[1].height;

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

            let cursor = if diff_focused {
                Some(self.main_panel.cursor_line)
            } else {
                None
            };
            let selection = self.main_panel.selection_range();
            let mut diff_widget =
                DiffViewWidget::new(self.main_panel.diff.as_ref(), self.main_panel.scroll_offset)
                    .block(diff_block);
            if let Some(c) = cursor {
                diff_widget = diff_widget.cursor_line(c);
            }
            if let Some(s) = selection {
                diff_widget = diff_widget.selection(s);
            }
            frame.render_widget(diff_widget, main_chunks[1]);
        }

        // Status bar
        let bindings = match self.focus {
            ReviewFocus::FileList => vec![
                Binding::new("j/k", "navigate"),
                Binding::new("Enter/Tab", "diff view"),
                Binding::new("Space", "mark reviewed"),
                Binding::new("Esc/q", "back"),
            ],
            ReviewFocus::DiffView | ReviewFocus::DiffLeft => vec![
                Binding::new("j/k", "navigate"),
                Binding::new("C-d/C-u", "page"),
                Binding::new("v", "select"),
                Binding::new("Space", "mark reviewed"),
                Binding::new("n/N", "next/prev file"),
                Binding::new("Esc/q", "back"),
            ],
            ReviewFocus::DiffRight => vec![
                Binding::new("j/k", "navigate"),
                Binding::new("C-d/C-u", "page"),
                Binding::new("v", "select"),
                Binding::new("Space", "unreview"),
                Binding::new("Tab", "remaining"),
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

    fn default_diff_focus(&self) -> ReviewFocus {
        if self.remaining_panel.diff.is_some() {
            ReviewFocus::DiffLeft
        } else {
            ReviewFocus::DiffView
        }
    }

    fn active_panel(&self) -> &DiffPanelState {
        match self.focus {
            ReviewFocus::DiffLeft => &self.remaining_panel,
            ReviewFocus::DiffRight => &self.reviewed_panel,
            _ => &self.main_panel,
        }
    }

    fn active_panel_mut(&mut self) -> &mut DiffPanelState {
        match self.focus {
            ReviewFocus::DiffLeft => &mut self.remaining_panel,
            ReviewFocus::DiffRight => &mut self.reviewed_panel,
            _ => &mut self.main_panel,
        }
    }

    fn load_current_file_diff(&mut self, repository: &Repository) {
        let Some(file) = self.files.get(self.file_selected_index) else {
            self.main_panel.clear();
            self.remaining_panel.clear();
            self.reviewed_panel.clear();
            return;
        };

        if file.is_binary {
            self.main_panel.clear();
            self.remaining_panel.clear();
            self.reviewed_panel.clear();
            return;
        }

        let review_status = file.review_status.clone();
        let (file_path, old_path) = resolve_file_paths(file);

        if review_status == ReviewStatus::PartiallyReviewed {
            // Load M→T (remaining) and B→M (reviewed) for split view
            match diff::generate_partial_review_diffs(
                repository,
                self.commit_id,
                self.change_id,
                &file_path,
                old_path.as_deref(),
            ) {
                Ok(partial) => {
                    self.remaining_panel.load(partial.remaining);
                    self.reviewed_panel.load(partial.reviewed);
                }
                Err(e) => {
                    log::error!("failed to load partial diffs: {}", e);
                    self.remaining_panel.clear();
                    self.reviewed_panel.clear();
                }
            }
            // Don't need B→T in split mode
            self.main_panel.clear();
        } else {
            // Load B→T for single panel
            match diff::generate_single_file_diff(
                repository,
                self.commit_id,
                &file_path,
                old_path.as_deref(),
            ) {
                Ok(d) => {
                    self.main_panel.load(d);
                }
                Err(e) => {
                    log::error!("failed to load diff: {}", e);
                    self.main_panel.clear();
                }
            }
            self.remaining_panel.clear();
            self.reviewed_panel.clear();
        }

        // Adjust focus if split state changed
        match self.focus {
            ReviewFocus::DiffLeft | ReviewFocus::DiffRight
                if self.remaining_panel.diff.is_none() =>
            {
                self.focus = ReviewFocus::DiffView;
            }
            ReviewFocus::DiffView if self.remaining_panel.diff.is_some() => {
                self.focus = ReviewFocus::DiffLeft;
            }
            _ => {}
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

    fn mark_lines_reviewed(&mut self, repository: &Repository) -> Result<(), String> {
        let panel = self.active_panel();
        let Some(hunk_id) = panel.compute_selected_hunk_id() else {
            self.active_panel_mut().cancel_selection();
            return Ok(());
        };
        let restore_pos = panel
            .selection_range()
            .map_or(panel.cursor_line, |(s, _)| s);

        let Some(file) = self.files.get(self.file_selected_index) else {
            return Ok(());
        };
        let (file_path, old_path) = resolve_file_paths(file);

        {
            let mut marker =
                marker_commit::MarkerCommit::get(repository, self.change_id, self.commit_id)
                    .map_err(|e| format!("Failed to open marker commit: {}", e))?;

            log::info!("marking hunk reviewed: {:?}", hunk_id);
            marker
                .mark_hunk_reviewed(&file_path, old_path.as_deref(), &hunk_id)
                .map_err(|e| format!("Failed to mark hunk: {}", e))?;

            let marker_id = marker
                .write()
                .map_err(|e| format!("Failed to write: {}", e))?;
            log::info!("marker commit written: {}", marker_id);
        }

        self.reload_file_list(repository)?;
        self.load_current_file_diff(repository);

        let vh = self.diff_view_height;
        self.active_panel_mut().set_cursor(restore_pos, vh);
        Ok(())
    }

    fn unmark_lines_reviewed(&mut self, repository: &Repository) -> Result<(), String> {
        let Some(hunk_id) = self.reviewed_panel.compute_selected_hunk_id() else {
            self.reviewed_panel.cancel_selection();
            return Ok(());
        };
        let restore_pos = self
            .reviewed_panel
            .selection_range()
            .map_or(self.reviewed_panel.cursor_line, |(s, _)| s);

        let Some(file) = self.files.get(self.file_selected_index) else {
            return Ok(());
        };
        let (file_path, old_path) = resolve_file_paths(file);

        {
            let mut marker =
                marker_commit::MarkerCommit::get(repository, self.change_id, self.commit_id)
                    .map_err(|e| format!("Failed to open marker commit: {}", e))?;

            log::info!("unmarking hunk reviewed: {:?}", hunk_id);
            marker
                .unmark_hunk_reviewed(&file_path, old_path.as_deref(), &hunk_id)
                .map_err(|e| format!("Failed to unmark hunk: {}", e))?;

            let marker_id = marker
                .write()
                .map_err(|e| format!("Failed to write: {}", e))?;
            log::info!("marker commit written: {}", marker_id);
        }

        self.reload_file_list(repository)?;
        self.load_current_file_diff(repository);

        let vh = self.diff_view_height;
        self.reviewed_panel.set_cursor(restore_pos, vh);
        Ok(())
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
        self.load_current_file_diff(repository);
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
