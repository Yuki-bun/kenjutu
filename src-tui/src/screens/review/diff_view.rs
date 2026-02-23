use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use git2::Repository;
use kenjutu_core::services::diff;
use kenjutu_types::{ChangeId, CommitId};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

use super::diff_panel::{DiffPanel, DiffPanelOutcome, MarkHunkReviewed, UnmarkHunkReviewed};

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

        // Always start on the left panel; fall back to right if left is empty.
        self.focus = self.default_focus();
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

    pub fn handle_key_event(&mut self, key: KeyEvent, repository: &Repository) -> DiffViewOutcome {
        // Let the focused panel handle navigation/selection keys first.
        if self.focused_panel_mut().handle_key_event(key) == DiffPanelOutcome::Consumed {
            return DiffViewOutcome::Continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => DiffViewOutcome::ExitToFileList,
            KeyCode::Tab => {
                if self.is_split() && self.focus == DiffSide::Left {
                    self.focus = DiffSide::Right;
                } else {
                    return DiffViewOutcome::ExitToFileList;
                }
                DiffViewOutcome::Continue
            }
            KeyCode::BackTab => {
                if self.focus == DiffSide::Right {
                    self.focus = DiffSide::Left;
                } else {
                    return DiffViewOutcome::ExitToFileList;
                }
                DiffViewOutcome::Continue
            }
            KeyCode::Char(' ') => {
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
                    Ok(true) => DiffViewOutcome::ActionApplied,
                    Ok(false) => DiffViewOutcome::Continue,
                    Err(e) => DiffViewOutcome::Error(e.to_string()),
                }
            }
            KeyCode::Char('n') => DiffViewOutcome::NextFile,
            KeyCode::Char('N') => DiffViewOutcome::PrevFile,
            _ => DiffViewOutcome::Continue,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, file_title: &str) {
        if self.is_split() {
            let diff_chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

            render_panel(
                frame,
                &mut self.left,
                diff_chunks[0],
                focused && self.focus == DiffSide::Left,
                " Remaining (M\u{2192}T) ",
                Borders::RIGHT,
            );

            render_panel(
                frame,
                &mut self.right,
                diff_chunks[1],
                focused && self.focus == DiffSide::Right,
                " Reviewed (B\u{2192}M) ",
                Borders::NONE,
            );
        } else {
            let (panel, is_focused) = match self.sole_panel() {
                Some(DiffSide::Right) => {
                    (&mut self.right, focused && self.focus == DiffSide::Right)
                }
                _ => (&mut self.left, focused && self.focus == DiffSide::Left),
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
    panel: &mut DiffPanel,
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

    panel.render(frame, area, block, focused);
}
