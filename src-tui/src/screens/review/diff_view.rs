use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git2::Repository;
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

use super::diff_panel::{DiffPanel, MarkHunkReviewed, UnmarkHunkReviewed};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Left,
    Right,
}

pub enum DiffViewOutcome {
    Continue,
    ExitToFileList,
    NextFile,
    PrevFile,
    ActionApplied,
    Error(String),
}

pub struct DiffView {
    left: DiffPanel,
    right: DiffPanel,
    focus: DiffSide,
    change_id: ChangeId,
    commit_id: CommitId,
    file_path: PathBuf,
    old_path: Option<PathBuf>,
    view_height: u16,
    pending_restore_pos: Option<usize>,
}

impl DiffView {
    pub fn new(change_id: ChangeId, commit_id: CommitId) -> Self {
        Self {
            left: DiffPanel::new(Box::new(MarkHunkReviewed)),
            right: DiffPanel::new(Box::new(UnmarkHunkReviewed)),
            focus: DiffSide::Left,
            change_id,
            commit_id,
            file_path: PathBuf::new(),
            old_path: None,
            view_height: 0,
            pending_restore_pos: None,
        }
    }

    pub fn load(
        &mut self,
        repository: &Repository,
        file_path: &std::path::Path,
        old_path: Option<&std::path::Path>,
    ) {
        self.file_path = file_path.to_path_buf();
        self.old_path = old_path.map(|p| p.to_path_buf());

        match diff::generate_partial_review_diffs(
            repository,
            self.commit_id,
            self.change_id,
            file_path,
            old_path,
        ) {
            Ok(partial) => {
                self.left.load(partial.remaining);
                self.right.load(partial.reviewed);
            }
            Err(e) => {
                log::error!("failed to load partial diffs: {}", e);
                self.left.clear();
                self.right.clear();
            }
        }

        // Adjust focus to a side that has content
        if !self.focused_panel().has_content() {
            self.focus = self.default_focus();
        }

        // Apply pending restore from a previous action
        if let Some(pos) = self.pending_restore_pos.take() {
            let vh = self.view_height;
            self.focused_panel_mut().set_cursor(pos, vh);
        }
    }

    pub fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
    }

    pub fn is_split(&self) -> bool {
        self.left.has_content() && self.right.has_content()
    }

    fn sole_panel(&self) -> Option<DiffSide> {
        match (self.left.has_content(), self.right.has_content()) {
            (true, false) => Some(DiffSide::Left),
            (false, true) => Some(DiffSide::Right),
            _ => None,
        }
    }

    fn default_focus(&self) -> DiffSide {
        if self.left.has_content() {
            DiffSide::Left
        } else {
            DiffSide::Right
        }
    }

    fn focused_panel(&self) -> &DiffPanel {
        match self.focus {
            DiffSide::Left => &self.left,
            DiffSide::Right => &self.right,
        }
    }

    fn focused_panel_mut(&mut self) -> &mut DiffPanel {
        match self.focus {
            DiffSide::Left => &mut self.left,
            DiffSide::Right => &mut self.right,
        }
    }

    pub fn action_label(&self) -> &'static str {
        self.focused_panel().action_label()
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        view_height: u16,
        repository: &Repository,
    ) -> DiffViewOutcome {
        self.view_height = view_height;
        let vh = view_height;

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.focused_panel().selection_active {
                    self.focused_panel_mut().cancel_selection();
                } else {
                    return DiffViewOutcome::ExitToFileList;
                }
            }
            KeyCode::Tab => {
                if self.is_split() && self.focus == DiffSide::Left {
                    self.focus = DiffSide::Right;
                } else {
                    return DiffViewOutcome::ExitToFileList;
                }
            }
            KeyCode::BackTab => {
                if self.focus == DiffSide::Right {
                    self.focus = DiffSide::Left;
                } else {
                    return DiffViewOutcome::ExitToFileList;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.focused_panel_mut().move_cursor(1, vh);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.focused_panel_mut().move_cursor(-1, vh);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half = (vh as i32 / 2).max(1);
                self.focused_panel_mut().move_cursor(half, vh);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half = (vh as i32 / 2).max(1);
                self.focused_panel_mut().move_cursor(-half, vh);
            }
            KeyCode::Char('g') => {
                self.focused_panel_mut().set_cursor(0, vh);
            }
            KeyCode::Char('G') => {
                let max = self.focused_panel().total_lines.saturating_sub(1);
                self.focused_panel_mut().set_cursor(max, vh);
            }
            KeyCode::Char('v') => {
                self.focused_panel_mut().toggle_selection();
            }
            KeyCode::Char(' ') => {
                let panel = self.focused_panel();
                let restore_pos = panel
                    .selection_range()
                    .map_or(panel.cursor_line, |(s, _)| s);

                // Borrow the panel mutably and other fields immutably using
                // direct field access to satisfy the borrow checker.
                let panel = match self.focus {
                    DiffSide::Left => &mut self.left,
                    DiffSide::Right => &mut self.right,
                };
                match panel.apply_action(
                    repository,
                    self.change_id,
                    self.commit_id,
                    &self.file_path,
                    self.old_path.as_deref(),
                ) {
                    Ok(true) => {
                        self.pending_restore_pos = Some(restore_pos);
                        return DiffViewOutcome::ActionApplied;
                    }
                    Ok(false) => {}
                    Err(e) => return DiffViewOutcome::Error(e.to_string()),
                }
            }
            KeyCode::Char('n') => return DiffViewOutcome::NextFile,
            KeyCode::Char('N') => return DiffViewOutcome::PrevFile,
            _ => {}
        }

        DiffViewOutcome::Continue
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, file_title: &str) {
        self.view_height = area.height;

        if self.is_split() {
            let diff_chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

            render_panel(
                frame,
                &self.left,
                diff_chunks[0],
                focused && self.focus == DiffSide::Left,
                " Remaining (M\u{2192}T) ",
                Borders::RIGHT,
            );

            render_panel(
                frame,
                &self.right,
                diff_chunks[1],
                focused && self.focus == DiffSide::Right,
                " Reviewed (B\u{2192}M) ",
                Borders::NONE,
            );
        } else {
            let (panel, is_focused) = match self.sole_panel() {
                Some(DiffSide::Right) => (&self.right, focused && self.focus == DiffSide::Right),
                _ => (&self.left, focused && self.focus == DiffSide::Left),
            };

            render_panel(
                frame,
                panel,
                area,
                is_focused,
                &format!(" {} ", file_title),
                Borders::NONE,
            );
        }
    }
}

fn render_panel(
    frame: &mut Frame,
    panel: &DiffPanel,
    area: Rect,
    focused: bool,
    title: &str,
    borders: Borders,
) {
    let block_style = if focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(borders)
        .border_style(block_style)
        .title(title);

    let cursor = if focused {
        Some(panel.cursor_line)
    } else {
        None
    };
    let selection = panel.selection_range();
    let mut widget = panel.widget(block);
    if let Some(c) = cursor {
        widget = widget.cursor_line(c);
    }
    if let Some(s) = selection {
        widget = widget.selection(s);
    }
    frame.render_widget(widget, area);
}
