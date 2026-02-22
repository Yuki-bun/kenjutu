use kenjutu_core::models::{DiffLineType, FileDiff};

pub struct DiffPanelState {
    pub diff: Option<FileDiff>,
    pub scroll_offset: usize,
    pub total_lines: usize,
    pub cursor_line: usize,
    pub selection_active: bool,
    pub selection_anchor: usize,
}

impl Default for DiffPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffPanelState {
    pub fn new() -> Self {
        Self {
            diff: None,
            scroll_offset: 0,
            total_lines: 0,
            cursor_line: 0,
            selection_active: false,
            selection_anchor: 0,
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
    pub fn compute_selected_hunk_id(&self) -> Option<marker_commit::HunkId> {
        let diff = self.diff.as_ref()?;

        let (sel_start, sel_end) = if self.selection_active {
            let start = self.selection_anchor.min(self.cursor_line);
            let end = self.selection_anchor.max(self.cursor_line);
            (start, end)
        } else {
            (self.cursor_line, self.cursor_line)
        };

        let mut old_min: Option<u32> = None;
        let mut old_max: Option<u32> = None;
        let mut new_min: Option<u32> = None;
        let mut new_max: Option<u32> = None;
        let mut last_old: Option<u32> = None;
        let mut last_new: Option<u32> = None;
        let mut line_idx: usize = 0;

        for hunk in &diff.hunks {
            line_idx += 1; // hunk header
            let hunk_line_start = line_idx;
            let hunk_line_end = line_idx + hunk.lines.len();

            if hunk_line_start > sel_end || hunk_line_end <= sel_start {
                line_idx = hunk_line_end;
                continue;
            }

            for (i, dl) in hunk.lines.iter().enumerate() {
                let abs_idx = hunk_line_start + i;
                if abs_idx < sel_start {
                    if let Some(n) = dl.old_lineno {
                        last_old = Some(n);
                    }
                    if let Some(n) = dl.new_lineno {
                        last_new = Some(n);
                    }
                    continue;
                }
                if abs_idx > sel_end {
                    break;
                }
                match dl.line_type {
                    DiffLineType::Deletion => {
                        if let Some(n) = dl.old_lineno {
                            old_min = Some(old_min.map_or(n, |m: u32| m.min(n)));
                            old_max = Some(old_max.map_or(n, |m: u32| m.max(n)));
                        }
                    }
                    DiffLineType::Addition => {
                        if let Some(n) = dl.new_lineno {
                            new_min = Some(new_min.map_or(n, |m: u32| m.min(n)));
                            new_max = Some(new_max.map_or(n, |m: u32| m.max(n)));
                        }
                    }
                    _ => {
                        if let Some(n) = dl.old_lineno {
                            last_old = Some(n);
                        }
                        if let Some(n) = dl.new_lineno {
                            last_new = Some(n);
                        }
                    }
                }
            }

            line_idx = hunk_line_end;
        }

        if old_min.is_none() && new_min.is_none() {
            return None;
        }

        let (old_start, old_lines) = match (old_min, old_max) {
            (Some(min), Some(max)) => (min, max - min + 1),
            _ => (last_old.unwrap_or(0), 0),
        };
        let (new_start, new_lines) = match (new_min, new_max) {
            (Some(min), Some(max)) => (min, max - min + 1),
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

    fn make_panel(hunks: Vec<DiffHunk>) -> DiffPanelState {
        let diff = FileDiff {
            hunks,
            new_file_lines: 0,
        };
        let mut panel = DiffPanelState::new();
        panel.load(diff);
        panel
    }

    #[test]
    fn no_diff_loaded() {
        let panel = DiffPanelState::new();
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
        assert_eq!(id.old_start, 2);
        assert_eq!(id.old_lines, 1);
        assert_eq!(id.new_start, 8);
        assert_eq!(id.new_lines, 1);
    }
}
