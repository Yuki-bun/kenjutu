/// Identifies a hunk in a unified diff by its header coordinates.
/// Coordinates are 1-based, matching the `@@ -old_start,old_lines +new_start,new_lines @@` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkId {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
}

/// Apply a hunk from `diff(M→T)` to M.
///
/// Splices the T lines covered by the hunk into M, replacing the corresponding M lines.
/// `hunk` coordinates are in M/T space (as they appear in `diff(M, T)`).
pub(crate) fn apply_hunk(m_content: &str, t_content: &str, hunk: &HunkId) -> String {
    let m_lines = split_lines_inclusive(m_content);
    let t_lines = split_lines_inclusive(t_content);

    // When old_lines=0 the unified diff convention is that old_start is the line
    // *after which* to insert, so we take old_start lines from M before the splice.
    // Otherwise old_start is 1-based, so we take old_start-1 lines before.
    let m_before_end = if hunk.old_lines == 0 {
        hunk.old_start as usize
    } else {
        hunk.old_start as usize - 1
    };
    let m_after_start = m_before_end + hunk.old_lines as usize;

    let t_start = if hunk.new_lines == 0 {
        0
    } else {
        hunk.new_start as usize - 1
    };
    let t_end = t_start + hunk.new_lines as usize;

    let mut result = String::new();
    for line in &m_lines[..m_before_end] {
        result.push_str(line);
    }
    for line in &t_lines[t_start..t_end] {
        result.push_str(line);
    }
    for line in &m_lines[m_after_start..] {
        result.push_str(line);
    }
    result
}

/// Reverse a hunk from `diff(B→M)` out of M.
///
/// Splices the B lines covered by the hunk back into M, replacing the corresponding M lines.
/// `hunk` coordinates are in B/M space (as they appear in `diff(B, M)`):
/// `old_*` are B coordinates, `new_*` are M coordinates.
pub(crate) fn unapply_hunk(m_content: &str, b_content: &str, hunk: &HunkId) -> String {
    let m_lines = split_lines_inclusive(m_content);
    let b_lines = split_lines_inclusive(b_content);

    // new_* are M coordinates in diff(B→M)
    let m_before_end = if hunk.new_lines == 0 {
        hunk.new_start as usize
    } else {
        hunk.new_start as usize - 1
    };
    let m_after_start = m_before_end + hunk.new_lines as usize;

    // old_* are B coordinates in diff(B→M)
    let b_start = if hunk.old_lines == 0 {
        0
    } else {
        hunk.old_start as usize - 1
    };
    let b_end = b_start + hunk.old_lines as usize;

    let mut result = String::new();
    for line in &m_lines[..m_before_end] {
        result.push_str(line);
    }
    for line in &b_lines[b_start..b_end] {
        result.push_str(line);
    }
    for line in &m_lines[m_after_start..] {
        result.push_str(line);
    }
    result
}

/// Splits `s` into lines preserving their terminators (`\n` or `\r\n`).
/// Returns an empty vec for an empty string.
fn split_lines_inclusive(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split_inclusive('\n').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_hunk_modification() {
        // M and T differ only in line 2 (1-based).
        // diff(M→T): @@ -1,3 +1,3 @@ covers lines 1-3 with context.
        let m = "line1\nold2\nline3\n";
        let t = "line1\nnew2\nline3\n";
        let hunk = HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        assert_eq!(apply_hunk(m, t, &hunk), t);
    }

    #[test]
    fn apply_hunk_modification_preserves_untouched_lines() {
        // Multi-line file; hunk covers only the middle section.
        let m = "a\nb\nold\nd\ne\n";
        let t = "a\nb\nnew\nd\ne\n";
        // Hunk: @@ -2,3 +2,3 @@ (lines b, old/new, d)
        let hunk = HunkId {
            old_start: 2,
            old_lines: 3,
            new_start: 2,
            new_lines: 3,
        };
        assert_eq!(apply_hunk(m, t, &hunk), t);
        // Line "a" and "e" must be unchanged
        assert!(apply_hunk(m, t, &hunk).starts_with("a\n"));
        assert!(apply_hunk(m, t, &hunk).ends_with("e\n"));
    }

    #[test]
    fn apply_hunk_pure_addition() {
        // T inserts "new\n" after line 2 of M.
        // diff(M→T): @@ -2,0 +3,1 @@ (old_start=2, old_lines=0 → insert after line 2)
        let m = "line1\nline2\nline3\n";
        let t = "line1\nline2\nnew\nline3\n";
        let hunk = HunkId {
            old_start: 2,
            old_lines: 0,
            new_start: 3,
            new_lines: 1,
        };
        assert_eq!(apply_hunk(m, t, &hunk), t);
    }

    #[test]
    fn apply_hunk_pure_deletion() {
        // T removes "del\n" (line 2 of M), with surrounding context.
        // diff(M→T): @@ -1,3 +1,2 @@ (line1, -del, line3)
        let m = "line1\ndel\nline3\n";
        let t = "line1\nline3\n";
        let hunk = HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 2,
        };
        assert_eq!(apply_hunk(m, t, &hunk), t);
    }

    #[test]
    fn unapply_hunk_modification() {
        // Apply then unapply should restore M.
        let b = "line1\nold2\nline3\n";
        let t = "line1\nnew2\nline3\n";
        // After marking: M == T. Hunk in diff(B→M) == diff(B→T):
        // @@ -1,3 +1,3 @@
        let hunk = HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        let m_after_mark = apply_hunk(b, t, &hunk);
        assert_eq!(m_after_mark, t);

        let m_after_unmark = unapply_hunk(&m_after_mark, b, &hunk);
        assert_eq!(m_after_unmark, b);
    }

    #[test]
    fn unapply_hunk_pure_addition_in_m() {
        // diff(B→M): @@ -2,0 +3,1 @@ — M added "new\n" after line 2.
        // Unapply should remove it.
        let b = "line1\nline2\nline3\n";
        let m = "line1\nline2\nnew\nline3\n";
        let hunk = HunkId {
            old_start: 2,
            old_lines: 0,
            new_start: 3,
            new_lines: 1,
        };
        assert_eq!(unapply_hunk(m, b, &hunk), b);
    }

    // ── Partial hunk application ─────────────────────────────────────────
    //
    // These tests use a file with two separate change regions (hunk1 near the
    // top, hunk2 near the bottom, separated by enough context that git would
    // emit them as two distinct @@ hunks).  Applying only one hunk at a time
    // produces a "partial" M that is a mix of B and T content.

    // B / starting M:
    //   head / a1 / mid1 / mid2 / mid3 / b1 / tail   (7 lines)
    // T:
    //   head / A1 / mid1 / mid2 / mid3 / B1 / tail   (7 lines)
    //
    // diff(M→T) produces two hunks with 1-line context each:
    //   hunk1: @@ -1,3 +1,3 @@  (head, a1→A1, mid1)
    //   hunk2: @@ -5,3 +5,3 @@  (mid3, b1→B1, tail)

    const BASE: &str = "head\na1\nmid1\nmid2\nmid3\nb1\ntail\n";
    const TARGET: &str = "head\nA1\nmid1\nmid2\nmid3\nB1\ntail\n";

    fn hunk1() -> HunkId {
        HunkId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        }
    }
    fn hunk2() -> HunkId {
        HunkId {
            old_start: 5,
            old_lines: 3,
            new_start: 5,
            new_lines: 3,
        }
    }

    #[test]
    fn partial_apply_first_hunk_only() {
        // Applying only hunk1: region 1 becomes T, region 2 stays B.
        let result = apply_hunk(BASE, TARGET, &hunk1());
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "A1", "hunk1 applied: line 2 should be A1");
        assert_eq!(lines[5], "b1", "hunk2 not applied: line 6 should remain b1");
    }

    #[test]
    fn partial_apply_second_hunk_only() {
        // Applying only hunk2: region 2 becomes T, region 1 stays B.
        let result = apply_hunk(BASE, TARGET, &hunk2());
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "a1", "hunk1 not applied: line 2 should remain a1");
        assert_eq!(lines[5], "B1", "hunk2 applied: line 6 should be B1");
    }

    #[test]
    fn partial_apply_both_hunks_sequentially() {
        // Applying hunk1 then hunk2 (with M/T coords unchanged because the two
        // hunks don't overlap and the line count doesn't change) should reach T.
        let m_after_1 = apply_hunk(BASE, TARGET, &hunk1());
        let m_after_2 = apply_hunk(&m_after_1, TARGET, &hunk2());
        assert_eq!(m_after_2, TARGET);
    }

    #[test]
    fn partial_unapply_first_hunk() {
        // Mark hunk1, then unmark it: should restore to B.
        let m = apply_hunk(BASE, TARGET, &hunk1());
        // diff(B→M) for hunk1 has the same coords (old=B, new=M-after-hunk1).
        let restored = unapply_hunk(&m, BASE, &hunk1());
        assert_eq!(restored, BASE);
    }

    #[test]
    fn partial_unapply_second_hunk_leaves_first_applied() {
        // Mark both hunks, then unmark hunk2: hunk1 should stay applied.
        let m_both = apply_hunk(&apply_hunk(BASE, TARGET, &hunk1()), TARGET, &hunk2());
        assert_eq!(m_both, TARGET);

        // After applying both hunks, diff(B→M)==diff(B→T). Unapply hunk2.
        // hunk2 in diff(B→M=TARGET) space: old=B coords, new=TARGET coords → same as hunk2().
        let m_only_1 = unapply_hunk(&m_both, BASE, &hunk2());
        let lines: Vec<&str> = m_only_1.lines().collect();
        assert_eq!(
            lines[1], "A1",
            "hunk1 should still be applied after unapplying hunk2"
        );
        assert_eq!(lines[5], "b1", "hunk2 should be reverted");
    }
}
