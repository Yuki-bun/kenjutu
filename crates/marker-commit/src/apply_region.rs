/// Identifies a region in a diff by its header coordinates.
/// Coordinates are 1-based, matching the `@@ -old_start,old_lines +new_start,new_lines @@` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionId {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
}

/// Apply a region from `diff(M→T)` to M.
///
/// Splices the T lines covered by the region into M, replacing the corresponding M lines.
/// `region` coordinates are in M/T space (as they appear in `diff(M, T)`).
pub(crate) fn apply_region(m_content: &str, t_content: &str, region: &RegionId) -> String {
    let m_lines = split_lines_inclusive(m_content);
    let t_lines = split_lines_inclusive(t_content);

    // When old_lines=0 the unified diff convention is that old_start is the line
    // *after which* to insert, so we take old_start lines from M before the splice.
    // Otherwise old_start is 1-based, so we take old_start-1 lines before.
    let m_before_end = if region.old_lines == 0 {
        region.old_start as usize
    } else {
        region.old_start as usize - 1
    };
    let m_after_start = m_before_end + region.old_lines as usize;

    let t_start = if region.new_lines == 0 {
        0
    } else {
        region.new_start as usize - 1
    };
    let t_end = t_start + region.new_lines as usize;

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

/// Reverse a region from `diff(B→M)` out of M.
///
/// Splices the B lines covered by the region back into M, replacing the corresponding M lines.
/// `region` coordinates are in B/M space (as they appear in `diff(B, M)`):
/// `old_*` are B coordinates, `new_*` are M coordinates.
pub(crate) fn unapply_region(m_content: &str, b_content: &str, region: &RegionId) -> String {
    let m_lines = split_lines_inclusive(m_content);
    let b_lines = split_lines_inclusive(b_content);

    // new_* are M coordinates in diff(B→M)
    let m_before_end = if region.new_lines == 0 {
        region.new_start as usize
    } else {
        region.new_start as usize - 1
    };
    let m_after_start = m_before_end + region.new_lines as usize;

    // old_* are B coordinates in diff(B→M)
    let b_start = if region.old_lines == 0 {
        0
    } else {
        region.old_start as usize - 1
    };
    let b_end = b_start + region.old_lines as usize;

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
    fn apply_region_modification() {
        // M and T differ only in line 2 (1-based).
        // diff(M→T): @@ -1,3 +1,3 @@ covers lines 1-3 with context.
        let m = "line1\nold2\nline3\n";
        let t = "line1\nnew2\nline3\n";
        let region = RegionId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        assert_eq!(apply_region(m, t, &region), t);
    }

    #[test]
    fn apply_region_modification_preserves_untouched_lines() {
        // Multi-line file; region covers only the middle section.
        let m = "a\nb\nold\nd\ne\n";
        let t = "a\nb\nnew\nd\ne\n";
        // Region: @@ -2,3 +2,3 @@ (lines b, old/new, d)
        let region = RegionId {
            old_start: 2,
            old_lines: 3,
            new_start: 2,
            new_lines: 3,
        };
        assert_eq!(apply_region(m, t, &region), t);
        // Line "a" and "e" must be unchanged
        assert!(apply_region(m, t, &region).starts_with("a\n"));
        assert!(apply_region(m, t, &region).ends_with("e\n"));
    }

    #[test]
    fn apply_region_pure_addition() {
        // T inserts "new\n" after line 2 of M.
        // diff(M→T): @@ -2,0 +3,1 @@ (old_start=2, old_lines=0 → insert after line 2)
        let m = "line1\nline2\nline3\n";
        let t = "line1\nline2\nnew\nline3\n";
        let region = RegionId {
            old_start: 2,
            old_lines: 0,
            new_start: 3,
            new_lines: 1,
        };
        assert_eq!(apply_region(m, t, &region), t);
    }

    #[test]
    fn apply_region_pure_deletion() {
        // T removes "del\n" (line 2 of M), with surrounding context.
        // diff(M→T): @@ -1,3 +1,2 @@ (line1, -del, line3)
        let m = "line1\ndel\nline3\n";
        let t = "line1\nline3\n";
        let region = RegionId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 2,
        };
        assert_eq!(apply_region(m, t, &region), t);
    }

    #[test]
    fn unapply_region_modification() {
        // Apply then unapply should restore M.
        let b = "line1\nold2\nline3\n";
        let t = "line1\nnew2\nline3\n";
        // After marking: M == T. Region in diff(B→M) == diff(B→T):
        // @@ -1,3 +1,3 @@
        let region = RegionId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        };
        let m_after_mark = apply_region(b, t, &region);
        assert_eq!(m_after_mark, t);

        let m_after_unmark = unapply_region(&m_after_mark, b, &region);
        assert_eq!(m_after_unmark, b);
    }

    #[test]
    fn unapply_region_pure_addition_in_m() {
        // diff(B→M): @@ -2,0 +3,1 @@ — M added "new\n" after line 2.
        // Unapply should remove it.
        let b = "line1\nline2\nline3\n";
        let m = "line1\nline2\nnew\nline3\n";
        let region = RegionId {
            old_start: 2,
            old_lines: 0,
            new_start: 3,
            new_lines: 1,
        };
        assert_eq!(unapply_region(m, b, &region), b);
    }

    // ── Partial region application ───────────────────────────────────────
    //
    // These tests use a file with two separate change regions (region1 near the
    // top, region2 near the bottom, separated by enough context that git would
    // emit them as two distinct @@ regions).  Applying only one region at a time
    // produces a "partial" M that is a mix of B and T content.

    // B / starting M:
    //   head / a1 / mid1 / mid2 / mid3 / b1 / tail   (7 lines)
    // T:
    //   head / A1 / mid1 / mid2 / mid3 / B1 / tail   (7 lines)
    //
    // diff(M→T) produces two regions with 1-line context each:
    //   region1: @@ -1,3 +1,3 @@  (head, a1→A1, mid1)
    //   region2: @@ -5,3 +5,3 @@  (mid3, b1→B1, tail)

    const BASE: &str = "head\na1\nmid1\nmid2\nmid3\nb1\ntail\n";
    const TARGET: &str = "head\nA1\nmid1\nmid2\nmid3\nB1\ntail\n";

    fn region1() -> RegionId {
        RegionId {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
        }
    }
    fn region2() -> RegionId {
        RegionId {
            old_start: 5,
            old_lines: 3,
            new_start: 5,
            new_lines: 3,
        }
    }

    #[test]
    fn partial_apply_first_region_only() {
        // Applying only region1: region 1 becomes T, region 2 stays B.
        let result = apply_region(BASE, TARGET, &region1());
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "A1", "region1 applied: line 2 should be A1");
        assert_eq!(
            lines[5], "b1",
            "region2 not applied: line 6 should remain b1"
        );
    }

    #[test]
    fn partial_apply_second_region_only() {
        // Applying only region2: region 2 becomes T, region 1 stays B.
        let result = apply_region(BASE, TARGET, &region2());
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            lines[1], "a1",
            "region1 not applied: line 2 should remain a1"
        );
        assert_eq!(lines[5], "B1", "region2 applied: line 6 should be B1");
    }

    #[test]
    fn partial_apply_both_regions_sequentially() {
        // Applying region1 then region2 (with M/T coords unchanged because the two
        // regions don't overlap and the line count doesn't change) should reach T.
        let m_after_1 = apply_region(BASE, TARGET, &region1());
        let m_after_2 = apply_region(&m_after_1, TARGET, &region2());
        assert_eq!(m_after_2, TARGET);
    }

    #[test]
    fn partial_unapply_first_region() {
        // Mark region1, then unmark it: should restore to B.
        let m = apply_region(BASE, TARGET, &region1());
        // diff(B→M) for region1 has the same coords (old=B, new=M-after-region1).
        let restored = unapply_region(&m, BASE, &region1());
        assert_eq!(restored, BASE);
    }

    #[test]
    fn partial_unapply_second_region_leaves_first_applied() {
        // Mark both regions, then unmark region2: region1 should stay applied.
        let m_both = apply_region(&apply_region(BASE, TARGET, &region1()), TARGET, &region2());
        assert_eq!(m_both, TARGET);

        // After applying both regions, diff(B→M)==diff(B→T). Unapply region2.
        // region2 in diff(B→M=TARGET) space: old=B coords, new=TARGET coords → same as region2().
        let m_only_1 = unapply_region(&m_both, BASE, &region2());
        let lines: Vec<&str> = m_only_1.lines().collect();
        assert_eq!(
            lines[1], "A1",
            "region1 should still be applied after unapplying region2"
        );
        assert_eq!(lines[5], "b1", "region2 should be reverted");
    }
}
