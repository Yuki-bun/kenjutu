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

    /// Compute `marker_commit::HunkId`s covering the selected (or cursor) addition/deletion lines.
    ///
    /// Line indices include hunk headers (each hunk header is 1 line, then hunk.lines follow).
    /// Context-only selections produce no HunkIds.
    pub fn compute_selected_hunk_ids(&self) -> Vec<marker_commit::HunkId> {
        let Some(diff) = self.diff.as_ref() else {
            return Vec::new();
        };

        let (sel_start, sel_end) = if self.selection_active {
            let start = self.selection_anchor.min(self.cursor_line);
            let end = self.selection_anchor.max(self.cursor_line);
            (start, end)
        } else {
            (self.cursor_line, self.cursor_line)
        };

        let mut result = Vec::new();
        let mut line_idx: usize = 0;

        for hunk in &diff.hunks {
            // Skip hunk header
            line_idx += 1;

            let hunk_line_start = line_idx;
            let hunk_line_end = line_idx + hunk.lines.len(); // exclusive

            // Check overlap with selection
            if hunk_line_start > sel_end || hunk_line_end <= sel_start {
                line_idx = hunk_line_end;
                continue;
            }

            // Gather selected addition/deletion lines with their line numbers
            let mut old_lines_in_sel: Vec<u32> = Vec::new();
            let mut new_lines_in_sel: Vec<u32> = Vec::new();
            let mut has_change = false;

            // Track the "edge" line numbers for computing insertion points
            let mut last_old_before: Option<u32> = None;
            let mut last_new_before: Option<u32> = None;

            for (i, dl) in hunk.lines.iter().enumerate() {
                let abs_idx = hunk_line_start + i;

                if abs_idx < sel_start {
                    if let Some(n) = dl.old_lineno {
                        last_old_before = Some(n);
                    }
                    if let Some(n) = dl.new_lineno {
                        last_new_before = Some(n);
                    }
                    continue;
                }
                if abs_idx > sel_end {
                    break;
                }

                match dl.line_type {
                    DiffLineType::Addition | DiffLineType::Deletion => {
                        has_change = true;
                    }
                    DiffLineType::Context => {}
                    _ => continue,
                }

                if let Some(n) = dl.old_lineno {
                    old_lines_in_sel.push(n);
                }
                if let Some(n) = dl.new_lineno {
                    new_lines_in_sel.push(n);
                }
            }

            if !has_change {
                line_idx = hunk_line_end;
                continue;
            }

            let (old_start, old_lines) = if old_lines_in_sel.is_empty() {
                // Pure additions: old_start = line after which to insert, old_lines = 0
                let insert_after = last_old_before.unwrap_or(hunk.old_start.saturating_sub(1));
                (insert_after, 0u32)
            } else {
                let first = *old_lines_in_sel.first().unwrap();
                let last = *old_lines_in_sel.last().unwrap();
                (first, last - first + 1)
            };

            let (new_start, new_lines) = if new_lines_in_sel.is_empty() {
                // Pure deletions: new_start = insertion point, new_lines = 0
                let insert_after = last_new_before.unwrap_or(hunk.new_start.saturating_sub(1));
                (insert_after, 0u32)
            } else {
                let first = *new_lines_in_sel.first().unwrap();
                let last = *new_lines_in_sel.last().unwrap();
                (first, last - first + 1)
            };

            result.push(marker_commit::HunkId {
                old_start,
                old_lines,
                new_start,
                new_lines,
            });

            line_idx = hunk_line_end;
        }

        result
    }
}
