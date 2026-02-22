use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct Binding {
    pub key: &'static str,
    pub description: &'static str,
}

impl Binding {
    pub fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

pub struct StatusBarWidget<'a> {
    bindings: &'a [Binding],
}

impl<'a> StatusBarWidget<'a> {
    pub fn new(bindings: &'a [Binding]) -> Self {
        Self { bindings }
    }
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = Vec::new();

        for (i, Binding { key, description }) in self.bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", Style::default()));
            }
            spans.push(Span::styled(
                *key,
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(60, 60, 60))
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", description),
                Style::default().fg(Color::Gray),
            ));
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}
