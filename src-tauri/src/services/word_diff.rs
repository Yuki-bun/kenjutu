use std::collections::BTreeMap;

use similar::{capture_diff_slices, Algorithm, ChangeTag, DiffOp, TextDiff};

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

/// Compute a similarity ratio between two strings (0.0 = nothing in common, 1.0 = identical).
fn line_similarity(a: &str, b: &str) -> f32 {
    let diff = TextDiff::from_words(a, b);
    diff.ratio()
}

/// Align old and new lines by content using Myers diff, returning index pairs
/// that should be word-diffed. Only lines within `Replace` regions are paired;
/// pure inserts, deletes, and equal lines are skipped.
/// Within Replace regions, lines are matched greedily by highest similarity.
fn match_block_lines(old_lines: &[SideLine], new_lines: &[SideLine]) -> Vec<(usize, usize)> {
    let old_contents: Vec<&str> = old_lines.iter().map(|l| l.content.as_str()).collect();
    let new_contents: Vec<&str> = new_lines.iter().map(|l| l.content.as_str()).collect();

    let ops = capture_diff_slices(Algorithm::Myers, &old_contents, &new_contents);

    let mut pairs = Vec::new();
    for op in ops {
        if let DiffOp::Replace {
            old_index,
            old_len,
            new_index,
            new_len,
        } = op
        {
            // For small Replace regions, find best matches by similarity.
            // Build all candidate pairs with their similarity scores, then
            // greedily pick the best unused pair in order-preserving fashion.
            let mut candidates: Vec<(usize, usize, f32)> = Vec::new();
            for oi in 0..old_len {
                for ni in 0..new_len {
                    let sim =
                        line_similarity(old_contents[old_index + oi], new_contents[new_index + ni]);
                    candidates.push((oi, ni, sim));
                }
            }
            // Sort by descending similarity
            candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

            let mut used_old = vec![false; old_len];
            let mut used_new = vec![false; new_len];
            let mut region_pairs: Vec<(usize, usize)> = Vec::new();

            for (oi, ni, sim) in &candidates {
                if used_old[*oi] || used_new[*ni] {
                    continue;
                }
                // Only pair lines that have meaningful similarity
                if *sim < 0.25 {
                    break;
                }
                used_old[*oi] = true;
                used_new[*ni] = true;
                region_pairs.push((old_index + oi, new_index + ni));
            }

            // Sort by old index to maintain order
            region_pairs.sort();
            pairs.extend(region_pairs);
        }
    }
    pairs
}

pub fn compute_word_diff(source: &impl HunkLines) -> WordDiffResult {
    let mut deletions: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let mut insertions: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();

    for block in source.blocks() {
        let pairs = match_block_lines(&block.old_lines, &block.new_lines);
        for (old_idx, new_idx) in pairs {
            let old_line = &block.old_lines[old_idx];
            let new_line = &block.new_lines[new_idx];
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
            !r.old_ranges
                .iter()
                .any(|&(s, e)| old[s..e].contains("TextDiff")),
            "TextDiff should not be marked as changed, got old_ranges: {:?}",
            r.old_ranges
                .iter()
                .map(|&(s, e)| &old[s..e])
                .collect::<Vec<_>>()
        );
        assert!(
            !r.new_ranges
                .iter()
                .any(|&(s, e)| new[s..e].contains("TextDiff")),
            "TextDiff should not be marked as changed, got new_ranges: {:?}",
            r.new_ranges
                .iter()
                .map(|&(s, e)| &new[s..e])
                .collect::<Vec<_>>()
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
    fn word_diff_zero_pair() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(10, "aaa"), line(11, "ccc")],
                new_lines: vec![line(20, "bbb"), line(21, "ccc")],
            }],
        };
        let result = compute_word_diff(&mock);
        // line 11/21 are identical — matched as Equal by the line-level diff.
        // line 10 "aaa" is a pure delete, line 20 "bbb" is a pure insert —
        // no word diff for either since they have no meaningful similarity.
        assert!(!result.deletions.contains_key(&10));
        assert!(!result.insertions.contains_key(&20));
        assert!(!result.deletions.contains_key(&11));
        assert!(!result.insertions.contains_key(&21));
    }

    #[test]
    fn word_diff_unequal_line_counts() {
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(1, "aaa bbb"), line(2, "ccc ddd")],
                new_lines: vec![line(1, "aaa zzz")],
            }],
        };
        // "aaa bbb" ↔ "aaa zzz" are similar enough to be paired;
        // "ccc ddd" is a pure delete with no pair.
        let result = compute_word_diff(&mock);
        assert!(result.deletions.contains_key(&1));
        assert!(result.insertions.contains_key(&1));
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

    #[test]
    fn word_diff_insertion_before_modification() {
        // Old has 1 line, new has 2 lines: an inserted line + a modified line.
        // Only the modified pair should get word diff, not the inserted line
        // paired with the wrong old line.
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(10, "hello world")],
                new_lines: vec![line(20, "brand new line"), line(21, "hello rust")],
            }],
        };
        let result = compute_word_diff(&mock);
        // The modified pair is old:10 "hello world" ↔ new:21 "hello rust"
        assert!(
            result.deletions.contains_key(&10),
            "old line 10 should have deletions (word 'world' changed)"
        );
        assert!(
            result.insertions.contains_key(&21),
            "new line 21 should have insertions (word 'rust' changed)"
        );
        // The inserted line 20 should NOT appear in insertions (it's a pure insert,
        // not word-diffed against anything)
        assert!(
            !result.insertions.contains_key(&20),
            "new line 20 is a pure insertion and should not have word-level diff"
        );
    }

    #[test]
    fn word_diff_deletion_before_modification() {
        // Old has 2 lines (a deleted line + a modified line), new has 1 line.
        // Only the modified pair should get word diff.
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(10, "deleted line"), line(11, "hello world")],
                new_lines: vec![line(20, "hello rust")],
            }],
        };
        let result = compute_word_diff(&mock);
        // The modified pair is old:11 "hello world" ↔ new:20 "hello rust"
        assert!(
            result.deletions.contains_key(&11),
            "old line 11 should have deletions (word 'world' changed)"
        );
        assert!(
            result.insertions.contains_key(&20),
            "new line 20 should have insertions (word 'rust' changed)"
        );
        // The deleted line 10 should NOT appear in deletions
        assert!(
            !result.deletions.contains_key(&10),
            "old line 10 is a pure deletion and should not have word-level diff"
        );
    }

    #[test]
    fn word_diff_equal_count_misaligned() {
        // 2 old, 2 new — positional zip would pair them wrong.
        // Old: ["aaa", "hello world"]
        // New: ["hello rust", "zzz"]
        // Positional zip would pair "aaa"↔"hello rust" and "hello world"↔"zzz",
        // but content-based alignment should pair "hello world"↔"hello rust".
        let mock = MockHunk {
            blocks: vec![Block {
                old_lines: vec![line(10, "aaa"), line(11, "hello world")],
                new_lines: vec![line(20, "hello rust"), line(21, "zzz")],
            }],
        };
        let result = compute_word_diff(&mock);
        // "hello world" ↔ "hello rust" should be word-diffed
        assert!(
            result.deletions.contains_key(&11),
            "old line 11 should have deletions for 'world'→'rust'"
        );
        assert!(
            result.insertions.contains_key(&20),
            "new line 20 should have insertions for 'world'→'rust'"
        );
        // "aaa" is a pure delete, "zzz" is a pure insert — no word diff
        assert!(
            !result.deletions.contains_key(&10),
            "old line 10 ('aaa') is a pure deletion, no word diff"
        );
        assert!(
            !result.insertions.contains_key(&21),
            "new line 21 ('zzz') is a pure insertion, no word diff"
        );
    }
}
