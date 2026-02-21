use std::sync::OnceLock;

use two_face::re_exports::syntect::easy::HighlightLines;
use two_face::re_exports::syntect::highlighting::{Color, Theme};
use two_face::re_exports::syntect::parsing::{SyntaxReference, SyntaxSet};

#[derive(Clone, Debug)]
pub struct Token {
    pub content: String,
    /// CSS hex color (e.g., "#cf222e"), None for default foreground
    pub color: Option<String>,
}

/// Global singleton for HighlightService to avoid repeated initialization.
static HIGHLIGHTER: OnceLock<HighlightService> = OnceLock::new();

pub struct HighlightService {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl HighlightService {
    pub fn global() -> &'static Self {
        HIGHLIGHTER.get_or_init(Self::new)
    }

    fn new() -> Self {
        let syntax_set = two_face::syntax::extra_newlines();
        let theme_set = two_face::theme::extra();

        // Use a theme that works well on colored backgrounds
        let theme = theme_set[two_face::theme::EmbeddedThemeName::Base16OceanDark].clone();

        Self { syntax_set, theme }
    }

    pub fn detect_syntax(&self, file_path: &str) -> Option<&SyntaxReference> {
        self.syntax_set
            .find_syntax_for_file(file_path)
            .unwrap_or(None)
    }

    pub fn default_syntax(&self) -> &SyntaxReference {
        self.syntax_set.find_syntax_plain_text()
    }

    pub fn parse_and_highlight<'a>(&'a self, syntax: &'a SyntaxReference) -> ParseAndHighlight<'a> {
        ParseAndHighlight::new(syntax, &self.theme)
    }
}

pub struct ParseAndHighlight<'a> {
    highlighter: HighlightLines<'a>,
}

impl<'a> ParseAndHighlight<'a> {
    fn new(syntax: &'a SyntaxReference, theme: &'a Theme) -> Self {
        let highlighter = HighlightLines::new(syntax, theme);
        Self { highlighter }
    }

    pub fn highlight_line(&mut self, line: &str) -> Vec<Token> {
        let res = self
            .highlighter
            .highlight_line(line, &HighlightService::global().syntax_set);
        let res = match res {
            Ok(v) => v,
            Err(err) => {
                log::error!("Highlighting error: {}", err);
                return vec![Token {
                    content: line.to_string(),
                    color: None,
                }];
            }
        };

        res.into_iter()
            .map(|(style, content)| Token {
                content: content.to_string(),
                color: Some(color_to_hex(style.foreground)),
            })
            .collect::<Vec<Token>>()
    }
}

fn color_to_hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}
