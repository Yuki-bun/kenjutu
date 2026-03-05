use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Parse a string containing ANSI SGR escape codes into a Ratatui `Line`
/// of styled `Span`s. Unrecognized sequences are silently skipped.
pub fn parse_ansi_line(raw: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        // Find next ESC
        let esc_start = memchr(0x1b, &bytes[pos..]).map(|i| pos + i);

        match esc_start {
            Some(esc) => {
                // Emit text before ESC
                if esc > pos {
                    let text = &raw[pos..esc];
                    spans.push(Span::styled(text.to_owned(), current_style));
                }

                // Parse CSI sequence: ESC [ <params> m
                if esc + 1 < len && bytes[esc + 1] == b'[' {
                    // Find the terminating 'm'
                    let seq_start = esc + 2;
                    let mut seq_end = seq_start;
                    while seq_end < len && bytes[seq_end] != b'm' {
                        seq_end += 1;
                    }
                    if seq_end < len {
                        let params_str = &raw[seq_start..seq_end];
                        current_style = apply_sgr(current_style, params_str);
                        pos = seq_end + 1;
                    } else {
                        // Malformed — skip ESC char
                        pos = esc + 1;
                    }
                } else {
                    // Not a CSI — skip ESC char
                    pos = esc + 1;
                }
            }
            None => {
                // No more ESC — emit remaining text
                let text = &raw[pos..];
                if !text.is_empty() {
                    spans.push(Span::styled(text.to_owned(), current_style));
                }
                break;
            }
        }
    }

    Line::from(spans)
}

/// Apply SGR (Select Graphic Rendition) parameters to an existing style.
fn apply_sgr(mut style: Style, params: &str) -> Style {
    let codes_u16: Vec<u16> = if params.is_empty() {
        vec![0]
    } else {
        params
            .split(';')
            .filter_map(|s| s.parse::<u16>().ok())
            .collect()
    };

    let mut i = 0;
    while i < codes_u16.len() {
        match codes_u16[i] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style = style.remove_modifier(Modifier::ITALIC),
            24 => style = style.remove_modifier(Modifier::UNDERLINED),
            27 => style = style.remove_modifier(Modifier::REVERSED),
            // Standard foreground 30-37
            c @ 30..=37 => style = style.fg(ansi_standard_color(c - 30)),
            38 => {
                if let Some(color) = parse_extended_color(&codes_u16, &mut i) {
                    style = style.fg(color);
                }
            }
            39 => style = style.fg(Color::Reset),
            // Standard background 40-47
            c @ 40..=47 => style = style.bg(ansi_standard_color(c - 40)),
            48 => {
                if let Some(color) = parse_extended_color(&codes_u16, &mut i) {
                    style = style.bg(color);
                }
            }
            49 => style = style.bg(Color::Reset),
            // Bright foreground 90-97
            c @ 90..=97 => style = style.fg(ansi_bright_color(c - 90)),
            // Bright background 100-107
            c @ 100..=107 => style = style.bg(ansi_bright_color(c - 100)),
            _ => {}
        }
        i += 1;
    }

    style
}

/// Parse extended color (256-color or truecolor) from SGR params.
/// Advances `i` past the consumed parameters.
fn parse_extended_color(codes: &[u16], i: &mut usize) -> Option<Color> {
    if *i + 1 < codes.len() && codes[*i + 1] == 5 && *i + 2 < codes.len() {
        // 256-color: 38;5;N or 48;5;N
        let n = codes[*i + 2];
        *i += 2;
        Some(Color::Indexed(n as u8))
    } else if *i + 1 < codes.len() && codes[*i + 1] == 2 && *i + 4 < codes.len() {
        // Truecolor: 38;2;R;G;B or 48;2;R;G;B
        let r = codes[*i + 2] as u8;
        let g = codes[*i + 3] as u8;
        let b = codes[*i + 4] as u8;
        *i += 4;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

/// Map standard ANSI color index (0-7) to ratatui Color.
fn ansi_standard_color(idx: u16) -> Color {
    match idx {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        _ => Color::Reset,
    }
}

/// Map bright ANSI color index (0-7) to ratatui Color.
fn ansi_bright_color(idx: u16) -> Color {
    match idx {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        7 => Color::White,
        _ => Color::Reset,
    }
}

/// Find the first occurrence of a byte in a slice (simple version).
fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// Strip ANSI escape codes from a string.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        if bytes[pos] == 0x1b && pos + 1 < len && bytes[pos + 1] == b'[' {
            // Skip to 'm'
            pos += 2;
            while pos < len && bytes[pos] != b'm' {
                pos += 1;
            }
            if pos < len {
                pos += 1; // skip 'm'
            }
        } else {
            result.push(s[pos..].chars().next().unwrap());
            pos += s[pos..].chars().next().unwrap().len_utf8();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_no_ansi() {
        let line = parse_ansi_line("hello world");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "hello world");
    }

    #[test]
    fn bold_red_text() {
        let line = parse_ansi_line("\x1b[1;31mERROR\x1b[0m: something");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "ERROR");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
        assert_eq!(line.spans[1].content, ": something");
    }

    #[test]
    fn strip_ansi_works() {
        let stripped = strip_ansi("\x1b[1;31mhello\x1b[0m world");
        assert_eq!(stripped, "hello world");
    }
}
