use similar::{ChangeTag, TextDiff};

use crate::models::{DiffLine, DiffLineType, HighlightToken};

/// Change ranges (byte offsets) for old and new lines.
struct InlineDiffRanges {
    old_ranges: Vec<(usize, usize)>,
    new_ranges: Vec<(usize, usize)>,
}

fn compute_inline_diff(old_line: &str, new_line: &str) -> InlineDiffRanges {
    let diff = TextDiff::from_words(old_line, new_line);

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

/// Check if a position falls within any of the change ranges.
fn is_in_change_range(pos: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| pos >= *start && pos < *end)
}

/// Find the next boundary (either end of current range or start of next range).
fn find_next_boundary(current_pos: usize, token_end: usize, ranges: &[(usize, usize)]) -> usize {
    let mut next = token_end;

    for (start, end) in ranges {
        // If we're before this range starts, the boundary is the start
        if *start > current_pos && *start < next {
            next = *start;
        }
        // If we're inside this range, the boundary is the end
        if current_pos >= *start && current_pos < *end && *end < next {
            next = *end;
        }
    }

    next
}

/// Apply change ranges to tokens, splitting them at change boundaries.
fn apply_change_ranges_to_tokens(
    tokens: Vec<HighlightToken>,
    change_ranges: &[(usize, usize)],
) -> Vec<HighlightToken> {
    if change_ranges.is_empty() {
        // No changes - mark all as unchanged
        return tokens
            .into_iter()
            .map(|t| HighlightToken {
                changed: false,
                ..t
            })
            .collect();
    }

    let mut result = Vec::new();
    let mut pos = 0usize;

    for token in tokens {
        let token_start = pos;
        let token_end = pos + token.content.len();
        let mut current_pos = token_start;

        while current_pos < token_end {
            let next_boundary = find_next_boundary(current_pos, token_end, change_ranges);
            let is_changed = is_in_change_range(current_pos, change_ranges);

            let slice_start = current_pos - token_start;
            let slice_end = next_boundary - token_start;

            if slice_end > slice_start {
                result.push(HighlightToken {
                    content: token.content[slice_start..slice_end].to_string(),
                    color: token.color.clone(),
                    changed: is_changed,
                });
            }

            current_pos = next_boundary;
        }

        pos = token_end;
    }

    result
}

fn tokens_to_string(tokens: &[HighlightToken]) -> String {
    tokens.iter().map(|t| t.content.as_str()).collect()
}

/// Apply word diff to paired lines within a hunk.
/// Modifies the lines in place, marking changed tokens.
pub fn apply_word_diff_to_hunk(lines: &mut [DiffLine]) {
    // First pass: mark all tokens as unchanged (default)
    for line in lines.iter_mut() {
        for token in line.tokens.iter_mut() {
            token.changed = false;
        }
    }

    // Find and process deletion-addition pairs
    let mut i = 0;
    while i < lines.len() {
        if lines[i].line_type == DiffLineType::Deletion {
            // Collect consecutive deletions
            let deletion_start = i;
            while i < lines.len() && lines[i].line_type == DiffLineType::Deletion {
                i += 1;
            }
            let deletion_end = i;

            // Collect consecutive additions
            let addition_start = i;
            while i < lines.len() && lines[i].line_type == DiffLineType::Addition {
                i += 1;
            }
            let addition_end = i;

            // Pair them up 1:1
            let deletions = deletion_end - deletion_start;
            let additions = addition_end - addition_start;
            let pairs = deletions.min(additions);

            for j in 0..pairs {
                let del_idx = deletion_start + j;
                let add_idx = addition_start + j;

                let old_content = tokens_to_string(&lines[del_idx].tokens);
                let new_content = tokens_to_string(&lines[add_idx].tokens);

                let diff_ranges = compute_inline_diff(&old_content, &new_content);

                // Apply changes to deletion line
                let del_tokens = std::mem::take(&mut lines[del_idx].tokens);
                lines[del_idx].tokens =
                    apply_change_ranges_to_tokens(del_tokens, &diff_ranges.old_ranges);

                // Apply changes to addition line
                let add_tokens = std::mem::take(&mut lines[add_idx].tokens);
                lines[add_idx].tokens =
                    apply_change_ranges_to_tokens(add_tokens, &diff_ranges.new_ranges);
            }
        } else {
            i += 1;
        }
    }
}
