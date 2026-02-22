use ratatui::style::Color;

/// Convert a CSS hex color string (e.g. "#cf222e") to a ratatui Color::Rgb.
pub fn css_hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_hex() {
        assert_eq!(css_hex_to_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(css_hex_to_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(css_hex_to_color("#0000ff"), Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn returns_none_for_invalid() {
        assert_eq!(css_hex_to_color("not a color"), None);
        assert_eq!(css_hex_to_color("#fff"), None);
        assert_eq!(css_hex_to_color(""), None);
    }
}
