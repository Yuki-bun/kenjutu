use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Outcome from handling a key event in the text input.
pub enum TextInputOutcome {
    /// Input is still active, keep rendering.
    Continue,
    /// User confirmed the input (pressed Enter).
    Confirm(String),
    /// User cancelled the input (pressed Esc).
    Cancel,
}

/// A single-line text input widget with cursor support.
pub struct TextInput {
    prompt: String,
    content: String,
    cursor: usize,
}

impl TextInput {
    pub fn new(prompt: &str, initial: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            content: initial.to_string(),
            cursor: initial.len(),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> TextInputOutcome {
        match key.code {
            KeyCode::Enter => TextInputOutcome::Confirm(self.content.clone()),
            KeyCode::Esc => TextInputOutcome::Cancel,
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'a' => self.cursor = 0,
                        'e' => self.cursor = self.content.len(),
                        'u' => {
                            self.content.drain(..self.cursor);
                            self.cursor = 0;
                        }
                        'k' => {
                            self.content.truncate(self.cursor);
                        }
                        'w' => {
                            // Delete word backwards
                            let new_cursor = self.prev_word_boundary();
                            self.content.drain(new_cursor..self.cursor);
                            self.cursor = new_cursor;
                        }
                        _ => {}
                    }
                } else {
                    self.content.insert(self.cursor, c);
                    self.cursor += c.len_utf8();
                }
                TextInputOutcome::Continue
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = self.prev_char_boundary();
                    self.content.drain(prev..self.cursor);
                    self.cursor = prev;
                }
                TextInputOutcome::Continue
            }
            KeyCode::Delete => {
                if self.cursor < self.content.len() {
                    let next = self.next_char_boundary();
                    self.content.drain(self.cursor..next);
                }
                TextInputOutcome::Continue
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor = self.prev_char_boundary();
                }
                TextInputOutcome::Continue
            }
            KeyCode::Right => {
                if self.cursor < self.content.len() {
                    self.cursor = self.next_char_boundary();
                }
                TextInputOutcome::Continue
            }
            KeyCode::Home => {
                self.cursor = 0;
                TextInputOutcome::Continue
            }
            KeyCode::End => {
                self.cursor = self.content.len();
                TextInputOutcome::Continue
            }
            _ => TextInputOutcome::Continue,
        }
    }

    fn prev_char_boundary(&self) -> usize {
        let mut pos = self.cursor.saturating_sub(1);
        while pos > 0 && !self.content.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn next_char_boundary(&self) -> usize {
        let mut pos = self.cursor + 1;
        while pos < self.content.len() && !self.content.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }

    fn prev_word_boundary(&self) -> usize {
        let bytes = self.content.as_bytes();
        let mut pos = self.cursor;
        // Skip trailing whitespace
        while pos > 0 && bytes[pos - 1] == b' ' {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && bytes[pos - 1] != b' ' {
            pos -= 1;
        }
        pos
    }

    /// Build the widget for rendering. Call this in `render()`.
    pub fn widget(&self) -> TextInputWidget<'_> {
        TextInputWidget { input: self }
    }
}

pub struct TextInputWidget<'a> {
    input: &'a TextInput,
}

impl<'a> Widget for TextInputWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let prompt_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let text_style = Style::default().fg(Color::White);
        let cursor_style = Style::default().fg(Color::Black).bg(Color::White);

        let prompt_span = Span::styled(&self.input.prompt, prompt_style);
        let prompt_width = self.input.prompt.len() as u16;

        // Calculate visible portion of text (scroll if needed)
        let available_width = area.width.saturating_sub(prompt_width) as usize;
        let cursor_in_content = self.input.cursor;

        // Simple scrolling: keep cursor visible
        let scroll_offset = if cursor_in_content >= available_width {
            cursor_in_content - available_width + 1
        } else {
            0
        };

        let visible_end = (scroll_offset + available_width).min(self.input.content.len());
        let visible_text = &self.input.content[scroll_offset..visible_end];
        let cursor_pos_in_visible = cursor_in_content - scroll_offset;

        // Split visible text at cursor position
        let (before_cursor, at_and_after) = visible_text.split_at(cursor_pos_in_visible);

        let mut spans = vec![prompt_span];
        if !before_cursor.is_empty() {
            spans.push(Span::styled(before_cursor, text_style));
        }

        // Render cursor character (or space if at end)
        if at_and_after.is_empty() {
            spans.push(Span::styled(" ", cursor_style));
        } else {
            let cursor_char_len = at_and_after
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            let (cursor_char, after_cursor) = at_and_after.split_at(cursor_char_len);
            spans.push(Span::styled(cursor_char, cursor_style));
            if !after_cursor.is_empty() {
                spans.push(Span::styled(after_cursor, text_style));
            }
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn initial_state() {
        let input = TextInput::new("prompt: ", "hello");
        assert_eq!(input.content, "hello");
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn typing_inserts_at_cursor() {
        let mut input = TextInput::new("> ", "");
        input.handle_key_event(key(KeyCode::Char('a')));
        input.handle_key_event(key(KeyCode::Char('b')));
        assert_eq!(input.content, "ab");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let mut input = TextInput::new("> ", "abc");
        input.handle_key_event(key(KeyCode::Backspace));
        assert_eq!(input.content, "ab");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn left_right_movement() {
        let mut input = TextInput::new("> ", "abc");
        input.handle_key_event(key(KeyCode::Left));
        assert_eq!(input.cursor, 2);
        input.handle_key_event(key(KeyCode::Left));
        assert_eq!(input.cursor, 1);
        input.handle_key_event(key(KeyCode::Right));
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn home_end() {
        let mut input = TextInput::new("> ", "abc");
        input.handle_key_event(key(KeyCode::Home));
        assert_eq!(input.cursor, 0);
        input.handle_key_event(key(KeyCode::End));
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn enter_confirms() {
        let mut input = TextInput::new("> ", "hello");
        let outcome = input.handle_key_event(key(KeyCode::Enter));
        assert!(matches!(outcome, TextInputOutcome::Confirm(s) if s == "hello"));
    }

    #[test]
    fn esc_cancels() {
        let mut input = TextInput::new("> ", "hello");
        let outcome = input.handle_key_event(key(KeyCode::Esc));
        assert!(matches!(outcome, TextInputOutcome::Cancel));
    }

    #[test]
    fn insert_in_middle() {
        let mut input = TextInput::new("> ", "ac");
        input.handle_key_event(key(KeyCode::Left)); // cursor at 1
        input.handle_key_event(key(KeyCode::Char('b')));
        assert_eq!(input.content, "abc");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn ctrl_a_moves_to_start() {
        let mut input = TextInput::new("> ", "hello");
        input.handle_key_event(ctrl_key('a'));
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn ctrl_e_moves_to_end() {
        let mut input = TextInput::new("> ", "hello");
        input.cursor = 0;
        input.handle_key_event(ctrl_key('e'));
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn ctrl_u_clears_before_cursor() {
        let mut input = TextInput::new("> ", "hello world");
        input.cursor = 5;
        input.handle_key_event(ctrl_key('u'));
        assert_eq!(input.content, " world");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn ctrl_k_clears_after_cursor() {
        let mut input = TextInput::new("> ", "hello world");
        input.cursor = 5;
        input.handle_key_event(ctrl_key('k'));
        assert_eq!(input.content, "hello");
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn delete_removes_at_cursor() {
        let mut input = TextInput::new("> ", "abc");
        input.cursor = 1;
        input.handle_key_event(key(KeyCode::Delete));
        assert_eq!(input.content, "ac");
        assert_eq!(input.cursor, 1);
    }
}
