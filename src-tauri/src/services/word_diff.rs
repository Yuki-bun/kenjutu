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
