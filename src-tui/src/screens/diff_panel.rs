use kenjutu_core::models::FileDiff;

pub struct DiffPanelState {
    pub diff: Option<FileDiff>,
    pub scroll_offset: usize,
    pub total_lines: usize,
    pub cursor_line: usize,
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
        }
    }

    pub fn load(&mut self, diff: FileDiff) {
        self.total_lines = diff.hunks.iter().map(|h| h.lines.len() + 1).sum();
        self.diff = Some(diff);
        self.scroll_offset = 0;
        self.cursor_line = 0;
    }

    pub fn clear(&mut self) {
        self.diff = None;
        self.scroll_offset = 0;
        self.total_lines = 0;
        self.cursor_line = 0;
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
}
