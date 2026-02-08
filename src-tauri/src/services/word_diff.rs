use std::{collections::BTreeMap, iter::zip};

use similar::{ChangeTag, TextDiff};

pub struct SideLine {
    pub lineno: u32,
    pub content: String,
}

pub struct Block {
    pub old_lines: Vec<SideLine>,
    pub new_lines: Vec<SideLine>,
}

pub trait HunkLines {
    fn blocks(&self) -> Vec<Block>;
}

/// Word-level change byte ranges, keyed by line number.
#[derive(Debug, Clone)]
pub struct WordDiffResult {
    /// old line number → byte ranges within that line that were deleted
    pub deletions: BTreeMap<u32, Vec<(usize, usize)>>,
    /// new line number → byte ranges within that line that were inserted
    pub insertions: BTreeMap<u32, Vec<(usize, usize)>>,
}

/// Change ranges (byte offsets) for old and new lines.
struct InlineDiffRanges {
    old_ranges: Vec<(usize, usize)>,
    new_ranges: Vec<(usize, usize)>,
}

/// Split a string into tokens on whitespace boundaries and punctuation.
/// Quote characters (`"`, `'`, `` ` ``) and delimiters (`:`, `(`, `)`)
/// each become their own token so that `similar` can match surrounding
/// content independently.
fn tokenize_words(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if matches!(c, '"' | '\'' | '`' | ':' | '(' | ')') {
            if i > start {
                tokens.push(&s[start..i]);
            }
            tokens.push(&s[i..i + 1]);
            start = i + 1;
        } else if c.is_whitespace() {
            if i > start && !s[start..i].chars().all(|ch| ch.is_whitespace()) {
                tokens.push(&s[start..i]);
                start = i;
            }
            // If we're already in whitespace, keep accumulating
        } else if i > start && s.as_bytes()[i - 1].is_ascii_whitespace() {
            // Transition from whitespace to non-whitespace
            tokens.push(&s[start..i]);
            start = i;
        }
    }
    if start < s.len() {
        tokens.push(&s[start..]);
    }
    tokens
}

fn compute_inline_diff(old_line: &str, new_line: &str) -> InlineDiffRanges {
    let old_tokens = tokenize_words(old_line);
    let new_tokens = tokenize_words(new_line);
    let diff = TextDiff::from_slices(&old_tokens, &new_tokens);

    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;

    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Delete => {
                old_ranges.push((old_pos, old_pos + len));
                old_pos += len;
            }
            ChangeTag::Insert => {
                new_ranges.push((new_pos, new_pos + len));
                new_pos += len;
            }
            ChangeTag::Equal => {
                old_pos += len;
                new_pos += len;
            }
        }
    }

    InlineDiffRanges {
        old_ranges,
        new_ranges,
    }
}

pub fn compute_word_diff(source: &impl HunkLines) -> WordDiffResult {
    let mut deletions: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let mut insertions: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();

    for block in source.blocks() {
        let pairs = zip(block.old_lines, block.new_lines);
        for (old_line, new_line) in pairs {
            let ranges = compute_inline_diff(&old_line.content, &new_line.content);
            if !ranges.old_ranges.is_empty() {
                deletions.insert(old_line.lineno, ranges.old_ranges);
            }
            if !ranges.new_ranges.is_empty() {
                insertions.insert(new_line.lineno, ranges.new_ranges);
            }
        }
    }

    WordDiffResult {
        deletions,
        insertions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compute_inline_diff tests ---

    #[test]
    fn inline_identical_strings() {
        let r = compute_inline_diff("hello world", "hello world");
        assert!(r.old_ranges.is_empty());
        assert!(r.new_ranges.is_empty());
    }

    #[test]
    fn inline_empty_strings() {
        let r = compute_inline_diff("", "");
        assert!(r.old_ranges.is_empty());
        assert!(r.new_ranges.is_empty());
    }

    #[test]
    fn inline_single_word_change() {
        let r = compute_inline_diff("hello world", "hello rust");
        assert_eq!(r.old_ranges, vec![(6, 11)]);
        assert_eq!(r.new_ranges, vec![(6, 10)]);
    }

    #[test]
    fn inline_word_insertion() {
        let r = compute_inline_diff("foo bar", "foo baz bar");
        assert!(r.old_ranges.is_empty());
        assert_eq!(r.new_ranges, vec![(4, 7), (7, 8)]);
    }

    #[test]
    fn inline_word_deletion() {
        let r = compute_inline_diff("foo baz bar", "foo bar");
        assert_eq!(r.old_ranges, vec![(4, 7), (7, 8)]);
        assert!(r.new_ranges.is_empty());
    }

    #[test]
    fn inline_quote_boundary() {
        // Adding classes inside a quoted string — the closing " should not
        // cause the unchanged class before it to be marked as changed.
        let old = r#"className="flex rounded data-[focused=true]:bg-accent/50""#;
        let new = r#"className="flex rounded data-[focused=true]:bg-accent/50 w-full text-left""#;
        let r = compute_inline_diff(old, new);
        // Only the added ` w-full text-left` portion should be marked
        assert!(r.old_ranges.is_empty());
    }

    #[test]
    fn inline_paren_and_colon_boundary() {
        // Changing a function name in a call — parens and colons should not
        // glue to adjacent tokens, so shared args stay matched.
        let old = "    let diff = TextDiff::from_words(old_line, new_line);";
        let new = "    let diff = TextDiff::from_slices(&old_tokens, &new_tokens);";
        let r = compute_inline_diff(old, new);
        // `let diff = TextDiff::` and `(` and `)` and `;` are shared;
        // only the function name and arguments should be marked.
        assert!(
            !r.old_ranges.iter().any(|&(s, e)| old[s..e].contains("TextDiff")),
            "TextDiff should not be marked as changed, got old_ranges: {:?}",
            r.old_ranges.iter().map(|&(s, e)| &old[s..e]).collect::<Vec<_>>()
        );
        assert!(
            !r.new_ranges.iter().any(|&(s, e)| new[s..e].contains("TextDiff")),
            "TextDiff should not be marked as changed, got new_ranges: {:?}",
            r.new_ranges.iter().map(|&(s, e)| &new[s..e]).collect::<Vec<_>>()
        );
    }

    #[test]
    fn inline_completely_different() {
        let r = compute_inline_diff("aaa", "zzz");
        assert_eq!(r.old_ranges, vec![(0, 3)]);
        assert_eq!(r.new_ranges, vec![(0, 3)]);
    }

    // --- compute_word_diff tests ---

    struct MockHunk {
        blocks: Vec<Block>,
    }

    impl HunkLines for MockHunk {
        fn blocks(&self) -> Vec<Block> {
            // Rebuild blocks since Block isn't Clone
            self.blocks
                .iter()
                .map(|b| Block {
                    old_lines: b
                        .old_lines
                        .iter()
                        .map(|l| SideLine {
                            lineno: l.lineno,
                            content: l.content.clone(),
                        })
                        .collect(),
                    new_lines: b
                        .new_lines
                        .iter()
                        .map(|l| SideLine {
                            lineno: l.lineno,
                            content: l.content.clone(),
                        })
                        .collect(),
                })
                .collect()
        }
    }

    fn line(lineno: u32, content: &str) -> SideLine {
        SideLine {
            lineno,
            content: content.to_string(),
        }
    }

    #[test]
    fn word_diff_single_pair() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(1, "hello world")],
                new_lines: vec![line(1, "hello rust")],
            }],
        };
        let result = compute_word_diff(&mock);
        assert!(result.deletions.contains_key(&1));
        assert!(result.insertions.contains_key(&1));
    }

    #[test]
    fn word_diff_multiple_pairs() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(10, "aaa"), line(11, "ccc")],
                new_lines: vec![line(20, "bbb"), line(21, "ccc")],
            }],
        };
        let result = compute_word_diff(&mock);
        // line 10/20: completely different
        assert!(result.deletions.contains_key(&10));
        assert!(result.insertions.contains_key(&20));
        // line 11/21: identical → no entries
        assert!(!result.deletions.contains_key(&11));
        assert!(!result.insertions.contains_key(&21));
    }

    #[test]
    fn word_diff_unequal_line_counts() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(1, "aaa"), line(2, "bbb")],
                new_lines: vec![line(1, "zzz")],
            }],
        };
        // zip truncates — only first pair processed, no panic
        let result = compute_word_diff(&mock);
        assert!(result.deletions.contains_key(&1));
        assert!(!result.deletions.contains_key(&2));
    }

    #[test]
    fn word_diff_identical_lines() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(1, "same content")],
                new_lines: vec![line(1, "same content")],
            }],
        };
        let result = compute_word_diff(&mock);
        assert!(result.deletions.is_empty());
        assert!(result.insertions.is_empty());
    }
}
