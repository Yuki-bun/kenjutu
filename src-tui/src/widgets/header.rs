use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct HeaderWidget<'a> {
    title: &'a str,
    change_id: &'a str,
    summary: &'a str,
}

impl<'a> HeaderWidget<'a> {
    pub fn new(title: &'a str, change_id: &'a str, summary: &'a str) -> Self {
        Self {
            title,
            change_id,
            summary,
        }
    }
}

impl<'a> Widget for HeaderWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let change_id_short = &self.change_id[..8.min(self.change_id.len())];
        let line = Line::from(vec![
            Span::styled(
                format!("{}: ", self.title),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{} ", change_id_short),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(self.summary, Style::default().fg(Color::White)),
        ]);

        let x = area.x;
        let y = area.y;
        buf.set_line(x, y, &line, area.width);
    }
}
