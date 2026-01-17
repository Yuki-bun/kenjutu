use crate::models::HighlightToken;
use two_face::re_exports::syntect::highlighting::{Color, Highlighter, Theme};
use two_face::re_exports::syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// Pre-highlighted file content stored as a Vec for efficient O(1) line lookup.
/// Index 0 = line 1, Index 1 = line 2, etc. (0-indexed storage, 1-indexed access)
pub struct HighlightedFile {
    lines: Vec<Vec<HighlightToken>>,
}

impl HighlightedFile {
    /// Get tokens for a 1-indexed line number.
    /// Returns None if line number is 0 or out of bounds.
    pub fn get(&self, lineno: u32) -> Option<&Vec<HighlightToken>> {
        if lineno == 0 {
            return None;
        }
        self.lines.get((lineno - 1) as usize)
    }

    /// Returns an empty HighlightedFile (for binary files or missing blobs)
    pub fn empty() -> Self {
        Self { lines: Vec::new() }
    }
}

pub struct HighlightService {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl HighlightService {
    /// Creates a new HighlightService with default syntaxes and theme.
    pub fn new() -> Self {
        let syntax_set = two_face::syntax::extra_newlines();
        let theme_set = two_face::theme::extra();

        // Use a theme that works well on colored backgrounds
        let theme = theme_set[two_face::theme::EmbeddedThemeName::Base16OceanDark].clone();

        Self { syntax_set, theme }
    }

    /// Detects the syntax for a file path using Syntect's built-in detection.
    /// Returns None if the language is not recognized.
    fn detect_syntax(&self, file_path: &str) -> Option<&SyntaxReference> {
        self.syntax_set
            .find_syntax_for_file(file_path)
            .unwrap_or(None)
    }

    /// Highlights the entire content of a file.
    /// Returns a HighlightedFile with tokens indexed by line number.
    ///
    /// Parameters:
    ///   - content: Full file content as a string
    ///   - file_path: Path used for language detection
    ///
    /// Returns: HighlightedFile (Vec-based, 1-indexed access via .get())
    pub fn highlight_file(&self, content: &str, file_path: &str) -> HighlightedFile {
        let syntax = match self.detect_syntax(file_path) {
            Some(s) => s,
            None => {
                // Unknown language: return plain tokens for each line
                let lines = content.lines().map(Self::plain_tokens).collect();
                return HighlightedFile { lines };
            }
        };

        let highlighter = Highlighter::new(&self.theme);
        let mut parse_state = ParseState::new(syntax);
        let mut highlight_state = two_face::re_exports::syntect::highlighting::HighlightState::new(
            &highlighter,
            ScopeStack::new(),
        );

        let lines = content
            .lines()
            .map(|line| {
                let ops = parse_state
                    .parse_line(line, &self.syntax_set)
                    .expect("Failed to parse line");
                let styled = two_face::re_exports::syntect::highlighting::HighlightIterator::new(
                    &mut highlight_state,
                    &ops,
                    line,
                    &highlighter,
                );

                styled
                    .map(|(style, text)| HighlightToken {
                        content: text.to_string(),
                        color: Some(color_to_hex(style.foreground)),
                    })
                    .collect()
            })
            .collect();

        HighlightedFile { lines }
    }

    /// Creates plain (unhighlighted) tokens for a single line.
    /// Used as fallback when language is unknown.
    fn plain_tokens(line: &str) -> Vec<HighlightToken> {
        vec![HighlightToken {
            content: line.to_string(),
            color: None,
        }]
    }
}

/// Converts syntect's Color to a CSS hex color string.
fn color_to_hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}
