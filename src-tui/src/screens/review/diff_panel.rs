use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;
use kenjutu_core::models::{DiffLineType, FileDiff};
use kenjutu_types::{ChangeId, CommitId};

use crate::widgets::diff_view::DiffViewWidget;
use ratatui::widgets::Block;

pub trait HunkReviewAction {
    fn apply(
        &self,
        marker: &mut marker_commit::MarkerCommit<'_>,
        file_path: &Path,
        old_path: Option<&Path>,
        hunk_id: &marker_commit::HunkId,
    ) -> Result<()>;

    fn label(&self) -> &'static str;
}

pub struct MarkHunkReviewed;

impl HunkReviewAction for MarkHunkReviewed {
    fn apply(
        &self,
        marker: &mut marker_commit::MarkerCommit<'_>,
        file_path: &Path,
        old_path: Option<&Path>,
        hunk_id: &marker_commit::HunkId,
    ) -> Result<()> {
        marker.mark_hunk_reviewed(file_path, old_path, hunk_id)?;
        Ok(())
    }

    fn label(&self) -> &'static str {
        "mark reviewed"
    }
}

pub struct UnmarkHunkReviewed;

impl HunkReviewAction for UnmarkHunkReviewed {
    fn apply(
        &self,
        marker: &mut marker_commit::MarkerCommit<'_>,
        file_path: &Path,
        old_path: Option<&Path>,
        hunk_id: &marker_commit::HunkId,
    ) -> Result<()> {
        marker.unmark_hunk_reviewed(file_path, old_path, hunk_id)?;
        Ok(())
    }

    fn label(&self) -> &'static str {
        "unreview"
    }
}

pub struct DiffPanel {
    pub(super) diff: Option<FileDiff>,
    pub(super) scroll_offset: usize,
    pub(super) total_lines: usize,
    pub(super) cursor_line: usize,
    pub(super) selection_active: bool,
    pub(super) selection_anchor: usize,
    action: Box<dyn HunkReviewAction>,
}

impl DiffPanel {
    pub fn new(action: Box<dyn HunkReviewAction>) -> Self {
        Self {
            diff: None,
            scroll_offset: 0,
            total_lines: 0,
            cursor_line: 0,
            selection_active: false,
            selection_anchor: 0,
            action,
        }
    }

    pub fn load(&mut self, diff: FileDiff) {
        self.total_lines = diff.hunks.iter().map(|h| h.lines.len() + 1).sum();
        self.diff = Some(diff);
        self.scroll_offset = 0;
        self.cursor_line = 0;
        self.selection_active = false;
        self.selection_anchor = 0;
    }

    pub fn clear(&mut self) {
        self.diff = None;
        self.scroll_offset = 0;
        self.total_lines = 0;
        self.cursor_line = 0;
        self.selection_active = false;
        self.selection_anchor = 0;
    }

    pub fn has_content(&self) -> bool {
        self.diff.as_ref().is_some_and(|d| !d.hunks.is_empty())
    }

    pub fn action_label(&self) -> &'static str {
        self.action.label()
    }

    pub fn apply_action(
        &mut self,
        repository: &Repository,
        change_id: ChangeId,
        commit_id: CommitId,
        file_path: &Path,
        old_path: Option<&Path>,
    ) -> Result<bool> {
        let Some(hunk_id) = self.compute_selected_hunk_id() else {
            self.cancel_selection();
            return Ok(false);
        };

        let mut marker = marker_commit::MarkerCommit::get(repository, change_id, commit_id)
            .context("Failed to open marker commit")?;

        log::info!("applying hunk action: {:?}", hunk_id);
        self.action
            .apply(&mut marker, file_path, old_path, &hunk_id)
            .context("Failed to apply hunk action")?;

        let marker_id = marker.write().context("Failed to write marker commit")?;
        log::info!("marker commit written: {}", marker_id);

        Ok(true)
    }

    pub fn widget<'a>(&'a self, block: Block<'a>) -> DiffViewWidget<'a> {
        DiffViewWidget::new(self.diff.as_ref(), self.scroll_offset).block(block)
    }

    pub fn ensure_cursor_visible(&mut self, view_height: u16) {
        let h = view_height as usize;
        if h == 0 {
            return;
        }
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + h {
            self.scroll_offset = self.cursor_line - h + 1;
        }
    }

    pub fn set_cursor(&mut self, pos: usize, view_height: u16) {
        let max = self.total_lines.saturating_sub(1);
        self.cursor_line = pos.min(max);
        self.ensure_cursor_visible(view_height);
    }

    pub fn move_cursor(&mut self, delta: i32, view_height: u16) {
        let max = self.total_lines.saturating_sub(1);
        let new_pos = if delta >= 0 {
            (self.cursor_line + delta as usize).min(max)
        } else {
            self.cursor_line.saturating_sub((-delta) as usize)
        };
        self.set_cursor(new_pos, view_height);
    }

    pub fn toggle_selection(&mut self) {
        if self.selection_active {
            self.selection_active = false;
        } else {
            self.selection_active = true;
            self.selection_anchor = self.cursor_line;
        }
    }

    pub fn cancel_selection(&mut self) {
        self.selection_active = false;
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        if !self.selection_active {
            return None;
        }
        let start = self.selection_anchor.min(self.cursor_line);
        let end = self.selection_anchor.max(self.cursor_line);
        Some((start, end))
    }

    /// Compute a `marker_commit::HunkId` covering the selected (or cursor) lines.
    ///
    /// Line indices include hunk headers (each hunk header is 1 line, then hunk.lines follow).
    ///
    /// Uses `line_type` (not the presence of `old_lineno`/`new_lineno`) to decide which
    /// side each line belongs to, because word-diff pairing can populate both line-number
    /// fields on additions and deletions.
    ///
    /// - Context + Deletion lines contribute to the old side.
    /// - Context + Addition lines contribute to the new side.
    /// - A selection containing only context lines returns `None` (no-op).
    pub fn compute_selected_hunk_id(&self) -> Option<marker_commit::HunkId> {
        let diff = self.diff.as_ref()?;

        let (sel_start, sel_end) = if self.selection_active {
            let start = self.selection_anchor.min(self.cursor_line);
            let end = self.selection_anchor.max(self.cursor_line);
            (start, end)
        } else {
            (self.cursor_line, self.cursor_line)
        };

        // Fallback for pure insertion / pure deletion (no old/new lines in selection).
        let mut last_old: Option<u32> = None;
        let mut last_new: Option<u32> = None;

        // First/last old_lineno from Context|Deletion lines in the selection.
        let mut first_old: Option<u32> = None;
        let mut last_old_in_sel: Option<u32> = None;
        // First/last new_lineno from Context|Addition lines in the selection.
        let mut first_new: Option<u32> = None;
        let mut last_new_in_sel: Option<u32> = None;

        let mut has_change = false;
        let mut line_idx: usize = 0;

        for hunk in &diff.hunks {
            line_idx += 1; // hunk header
            let hunk_line_start = line_idx;
            let hunk_line_end = line_idx + hunk.lines.len();

            if hunk_line_end <= sel_start {
                // Entire hunk is before the selection.
                if hunk.old_lines > 0 {
                    last_old = Some(hunk.old_start + hunk.old_lines - 1);
                }
                if hunk.new_lines > 0 {
                    last_new = Some(hunk.new_start + hunk.new_lines - 1);
                }
                line_idx = hunk_line_end;
                continue;
            }
            if hunk_line_start > sel_end {
                break;
            }

            for (i, dl) in hunk.lines.iter().enumerate() {
                let abs_idx = hunk_line_start + i;

                if abs_idx < sel_start {
                    // Before the selection — update fallbacks.
                    if matches!(dl.line_type, DiffLineType::Context | DiffLineType::Deletion) {
                        if let Some(n) = dl.old_lineno {
                            last_old = Some(n);
                        }
                    }
                    if matches!(dl.line_type, DiffLineType::Context | DiffLineType::Addition) {
                        if let Some(n) = dl.new_lineno {
                            last_new = Some(n);
                        }
                    }
                    continue;
                }
                if abs_idx > sel_end {
                    break;
                }

                // Line is inside the selection.
                if matches!(
                    dl.line_type,
                    DiffLineType::Addition | DiffLineType::Deletion
                ) {
                    has_change = true;
                }
                if matches!(dl.line_type, DiffLineType::Context | DiffLineType::Deletion) {
                    if let Some(n) = dl.old_lineno {
                        if first_old.is_none() {
                            first_old = Some(n);
                        }
                        last_old_in_sel = Some(n);
                    }
                }
                if matches!(dl.line_type, DiffLineType::Context | DiffLineType::Addition) {
                    if let Some(n) = dl.new_lineno {
                        if first_new.is_none() {
                            first_new = Some(n);
                        }
                        last_new_in_sel = Some(n);
                    }
                }
            }

            line_idx = hunk_line_end;
        }

        if !has_change {
            return None;
        }

        let (old_start, old_lines) = match (first_old, last_old_in_sel) {
            (Some(first), Some(last)) => (first, last - first + 1),
            _ => (last_old.unwrap_or(0), 0),
        };
        let (new_start, new_lines) = match (first_new, last_new_in_sel) {
            (Some(first), Some(last)) => (first, last - first + 1),
            _ => (last_new.unwrap_or(0), 0),
        };

        Some(marker_commit::HunkId {
            old_start,
            old_lines,
            new_start,
            new_lines,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kenjutu_core::models::{DiffHunk, DiffLine};

    fn ctx(old: u32, new: u32) -> DiffLine {
        DiffLine {
            line_type: DiffLineType::Context,
            old_lineno: Some(old),
            new_lineno: Some(new),
            tokens: vec![],
        }
    }

    fn add(new: u32) -> DiffLine {
        DiffLine {
            line_type: DiffLineType::Addition,
            old_lineno: None,
            new_lineno: Some(new),
            tokens: vec![],
        }
    }

    fn del(old: u32) -> DiffLine {
        DiffLine {
            line_type: DiffLineType::Deletion,
            old_lineno: Some(old),
            new_lineno: None,
            tokens: vec![],
        }
    }

    /// Deletion paired with an addition (word-diff sets both line numbers).
    fn paired_del(old: u32, new: u32) -> DiffLine {
        DiffLine {
            line_type: DiffLineType::Deletion,
            old_lineno: Some(old),
            new_lineno: Some(new),
            tokens: vec![],
        }
    }

    /// Addition paired with a deletion (word-diff sets both line numbers).
    fn paired_add(new: u32, old: u32) -> DiffLine {
        DiffLine {
            line_type: DiffLineType::Addition,
            old_lineno: Some(old),
            new_lineno: Some(new),
            tokens: vec![],
        }
    }

    fn hunk(
        old_start: u32,
        old_lines: u32,
        new_start: u32,
        new_lines: u32,
        lines: Vec<DiffLine>,
    ) -> DiffHunk {
        DiffHunk {
            old_start,
            old_lines,
            new_start,
            new_lines,
            header: String::new(),
            lines,
        }
    }

    fn make_panel(hunks: Vec<DiffHunk>) -> DiffPanel {
        let diff = FileDiff {
            hunks,
            new_file_lines: 0,
        };
        let mut panel = DiffPanel::new(Box::new(MarkHunkReviewed));
        panel.load(diff);
        panel
    }

    #[test]
    fn no_diff_loaded() {
        let panel = DiffPanel::new(Box::new(MarkHunkReviewed));
        assert_eq!(panel.compute_selected_hunk_id(), None);
    }

    #[test]
    fn cursor_on_hunk_header() {
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), del(2), add(2), ctx(3, 3)],
        )]);
        panel.cursor_line = 0; // hunk header
        assert_eq!(panel.compute_selected_hunk_id(), None);
    }

    #[test]
    fn cursor_on_context_line() {
        // Visual: 0=header, 1=ctx(1,1), 2=del(2), 3=add(2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), del(2), add(2), ctx(3, 3)],
        )]);
        panel.cursor_line = 1; // ctx(1,1)
        assert_eq!(panel.compute_selected_hunk_id(), None);
    }

    #[test]
    fn cursor_on_single_deletion() {
        // Visual: 0=header, 1=ctx(1,1), 2=del(2), 3=add(2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), del(2), add(2), ctx(3, 3)],
        )]);
        panel.cursor_line = 2; // del(2)
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 2);
        assert_eq!(id.old_lines, 1);
        assert_eq!(id.new_start, 1); // last_new from ctx(1,1)
        assert_eq!(id.new_lines, 0);
    }

    #[test]
    fn cursor_on_single_addition() {
        // Visual: 0=header, 1=ctx(1,1), 2=del(2), 3=add(2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), del(2), add(2), ctx(3, 3)],
        )]);
        panel.cursor_line = 3; // add(2)
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 2); // last_old from del(2)
        assert_eq!(id.old_lines, 0);
        assert_eq!(id.new_start, 2);
        assert_eq!(id.new_lines, 1);
    }

    #[test]
    fn selection_spanning_modification() {
        // Visual: 0=header, 1=ctx(1,1), 2=del(2), 3=add(2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), del(2), add(2), ctx(3, 3)],
        )]);
        panel.selection_active = true;
        panel.selection_anchor = 2; // del(2)
        panel.cursor_line = 3; // add(2)
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 2);
        assert_eq!(id.old_lines, 1);
        assert_eq!(id.new_start, 2);
        assert_eq!(id.new_lines, 1);
    }

    #[test]
    fn selection_spanning_multiple_additions() {
        // Visual: 0=header, 1=ctx(1,1), 2=add(2), 3=add(3), 4=ctx(2,4)
        let mut panel = make_panel(vec![hunk(
            1,
            2,
            1,
            4,
            vec![ctx(1, 1), add(2), add(3), ctx(2, 4)],
        )]);
        panel.selection_active = true;
        panel.selection_anchor = 2; // add(2)
        panel.cursor_line = 3; // add(3)
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 1); // last_old from ctx(1,1)
        assert_eq!(id.old_lines, 0);
        assert_eq!(id.new_start, 2);
        assert_eq!(id.new_lines, 2);
    }

    #[test]
    fn pure_addition_after_deletion_uses_last_old() {
        // Deletions shift line numbers: new_lineno - 1 != old line above.
        // Old: line1, DELETED, line3, line4
        // New: line1, line3, NEW, line4
        // Visual: 0=header, 1=ctx(1,1), 2=del(2), 3=ctx(3,2), 4=add(3), 5=ctx(4,4)
        let mut panel = make_panel(vec![hunk(
            1,
            4,
            1,
            4,
            vec![ctx(1, 1), del(2), ctx(3, 2), add(3), ctx(4, 4)],
        )]);
        panel.cursor_line = 4; // add(3)
        let id = panel.compute_selected_hunk_id().unwrap();
        // last_old = 3 from ctx(3,2), NOT new_min - 1 = 2
        assert_eq!(id.old_start, 3);
        assert_eq!(id.old_lines, 0);
        assert_eq!(id.new_start, 3);
        assert_eq!(id.new_lines, 1);
    }

    #[test]
    fn selection_across_two_hunks() {
        // Hunk1: 0=header, 1=ctx(1,1), 2=del(2), 3=ctx(3,2)
        // Hunk2: 4=header, 5=ctx(8,7), 6=add(8), 7=ctx(9,9)
        let mut panel = make_panel(vec![
            hunk(1, 3, 1, 2, vec![ctx(1, 1), del(2), ctx(3, 2)]),
            hunk(8, 2, 7, 3, vec![ctx(8, 7), add(8), ctx(9, 9)]),
        ]);
        panel.selection_active = true;
        panel.selection_anchor = 2; // del(2) in hunk1
        panel.cursor_line = 6; // add(8) in hunk2
        let id = panel.compute_selected_hunk_id().unwrap();
        // Covers the full span including context lines and the gap between hunks.
        assert_eq!(id.old_start, 2);
        assert_eq!(id.old_lines, 7); // del(2), ctx(3), ctx(8) → span 2..8
        assert_eq!(id.new_start, 2);
        assert_eq!(id.new_lines, 7); // ctx(2), ctx(7), add(8) → span 2..8
    }

    #[test]
    fn paired_deletion_ignores_new_lineno() {
        // Word-diff paired: del has new_lineno set, add has old_lineno set.
        // 0=header, 1=ctx(1,1), 2=paired_del(2,2), 3=paired_add(2,2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), paired_del(2, 2), paired_add(2, 2), ctx(3, 3)],
        )]);
        // Cursor on paired deletion only.
        panel.cursor_line = 2;
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 2);
        assert_eq!(id.old_lines, 1);
        // new_lines must be 0 — the paired new_lineno on the deletion is not counted.
        assert_eq!(id.new_lines, 0);
    }

    #[test]
    fn paired_addition_ignores_old_lineno() {
        // 0=header, 1=ctx(1,1), 2=paired_del(2,2), 3=paired_add(2,2), 4=ctx(3,3)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            3,
            vec![ctx(1, 1), paired_del(2, 2), paired_add(2, 2), ctx(3, 3)],
        )]);
        // Cursor on paired addition only.
        panel.cursor_line = 3;
        let id = panel.compute_selected_hunk_id().unwrap();
        // old_lines must be 0 — the paired old_lineno on the addition is not counted.
        assert_eq!(id.old_lines, 0);
        assert_eq!(id.old_start, 2); // last_old from paired_del before selection
        assert_eq!(id.new_start, 2);
        assert_eq!(id.new_lines, 1);
    }

    #[test]
    fn range_selection_includes_context() {
        // 0=header, 1=ctx(1,1), 2=del(2), 3=add(2), 4=add(3), 5=ctx(3,4)
        let mut panel = make_panel(vec![hunk(
            1,
            3,
            1,
            4,
            vec![ctx(1, 1), del(2), add(2), add(3), ctx(3, 4)],
        )]);
        // Select ctx(1,1) through add(3).
        panel.selection_active = true;
        panel.selection_anchor = 1;
        panel.cursor_line = 4;
        let id = panel.compute_selected_hunk_id().unwrap();
        assert_eq!(id.old_start, 1); // from ctx(1,1)
        assert_eq!(id.old_lines, 2); // ctx(old=1) + del(old=2) → span 1..2
        assert_eq!(id.new_start, 1); // from ctx(1,1)
        assert_eq!(id.new_lines, 3); // ctx(new=1), add(new=2), add(new=3) → span 1..3
    }
}
