import { describe, expect, it } from "vitest"

import { DiffHunk, DiffLine, DiffLineType } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import {
  computeRegionId,
  CursorPosition,
  getSelectedRegion,
  LineSelectionState,
} from "./useLineSelection"

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeLine(
  lineType: DiffLineType,
  oldLineno: number | null,
  newLineno: number | null,
): DiffLine {
  return { lineType, oldLineno, newLineno, tokens: [] }
}

function makeHunk(lines: DiffLine[]): DiffHunk {
  return {
    oldStart: 1,
    oldLines: lines.filter(
      (l) => l.lineType === "context" || l.lineType === "deletion",
    ).length,
    newStart: 1,
    newLines: lines.filter(
      (l) => l.lineType === "context" || l.lineType === "addition",
    ).length,
    header: "",
    lines,
  }
}

function wrapHunk(lines: DiffLine[]): DiffElement[] {
  return [{ type: "hunk", hunk: makeHunk(lines) }]
}

function sel(
  cursor: CursorPosition,
  anchor?: CursorPosition,
): LineSelectionState {
  return { cursor, anchor: anchor ?? null }
}

// ---------------------------------------------------------------------------
// getSelectedRegion
// ---------------------------------------------------------------------------

describe("getSelectedRegion", () => {
  it("returns nulls for null selection", () => {
    const elements = wrapHunk([makeLine("context", 1, 1)])
    expect(getSelectedRegion(null, elements)).toEqual({
      left: null,
      right: null,
    })
  })

  it("returns nulls when cursor points to nonexistent line", () => {
    const elements = wrapHunk([makeLine("context", 1, 1)])
    const result = getSelectedRegion(sel({ line: 99, side: "LEFT" }), elements)
    expect(result).toEqual({ left: null, right: null })
  })

  it("cursor on context line (no anchor) populates both sides", () => {
    const elements = wrapHunk([makeLine("context", 5, 5)])
    const result = getSelectedRegion(sel({ line: 5, side: "LEFT" }), elements)
    expect(result).toEqual({
      left: { start: 5, end: 5 },
      right: { start: 5, end: 5 },
    })
  })

  it("cursor on deletion (no anchor) populates left only", () => {
    const elements = wrapHunk([makeLine("deletion", 3, null)])
    const result = getSelectedRegion(sel({ line: 3, side: "LEFT" }), elements)
    expect(result).toEqual({
      left: { start: 3, end: 3 },
      right: null,
    })
  })

  it("cursor on addition (no anchor) populates right only", () => {
    const elements = wrapHunk([makeLine("addition", null, 7)])
    const result = getSelectedRegion(sel({ line: 7, side: "RIGHT" }), elements)
    expect(result).toEqual({
      left: null,
      right: { start: 7, end: 7 },
    })
  })

  it("anchor + cursor spanning context and deletions", () => {
    const lines = [
      makeLine("context", 1, 1),
      makeLine("deletion", 2, null),
      makeLine("deletion", 3, null),
      makeLine("context", 4, 2),
    ]
    const elements = wrapHunk(lines)
    const result = getSelectedRegion(
      sel({ line: 4, side: "LEFT" }, { line: 1, side: "LEFT" }),
      elements,
    )
    expect(result).toEqual({
      left: { start: 1, end: 4 },
      right: { start: 1, end: 2 },
    })
  })

  it("anchor + cursor spanning context and additions", () => {
    const lines = [
      makeLine("context", 1, 1),
      makeLine("addition", null, 2),
      makeLine("addition", null, 3),
      makeLine("context", 2, 4),
    ]
    const elements = wrapHunk(lines)
    const result = getSelectedRegion(
      sel({ line: 4, side: "RIGHT" }, { line: 1, side: "RIGHT" }),
      elements,
    )
    expect(result).toEqual({
      left: { start: 1, end: 2 },
      right: { start: 1, end: 4 },
    })
  })

  it("reversed anchor/cursor order produces same result", () => {
    const lines = [
      makeLine("context", 1, 1),
      makeLine("deletion", 2, null),
      makeLine("context", 3, 2),
    ]
    const elements = wrapHunk(lines)

    const forward = getSelectedRegion(
      sel({ line: 3, side: "LEFT" }, { line: 1, side: "LEFT" }),
      elements,
    )
    const reversed = getSelectedRegion(
      sel({ line: 1, side: "LEFT" }, { line: 3, side: "LEFT" }),
      elements,
    )
    expect(forward).toEqual(reversed)
  })

  it("cursor on addition among deletions produces right-only range", () => {
    const lines = [
      makeLine("deletion", 1, null),
      makeLine("deletion", 2, null),
      makeLine("addition", null, 1),
      makeLine("addition", null, 2),
    ]
    const elements = wrapHunk(lines)
    const result = getSelectedRegion(sel({ line: 1, side: "RIGHT" }), elements)
    expect(result).toEqual({
      left: null,
      right: { start: 1, end: 1 },
    })
  })

  it("multi-line selection across additions only", () => {
    const lines = [
      makeLine("addition", null, 1),
      makeLine("addition", null, 2),
      makeLine("addition", null, 3),
    ]
    const elements = wrapHunk(lines)
    const result = getSelectedRegion(
      sel({ line: 3, side: "RIGHT" }, { line: 1, side: "RIGHT" }),
      elements,
    )
    expect(result).toEqual({
      left: null,
      right: { start: 1, end: 3 },
    })
  })

  it("multi-line selection across deletions only", () => {
    const lines = [
      makeLine("deletion", 10, null),
      makeLine("deletion", 11, null),
      makeLine("deletion", 12, null),
    ]
    const elements = wrapHunk(lines)
    const result = getSelectedRegion(
      sel({ line: 12, side: "LEFT" }, { line: 10, side: "LEFT" }),
      elements,
    )
    expect(result).toEqual({
      left: { start: 10, end: 12 },
      right: null,
    })
  })

  it("works across multiple hunk elements", () => {
    const elements: DiffElement[] = [
      {
        type: "hunk",
        hunk: makeHunk([
          makeLine("context", 1, 1),
          makeLine("deletion", 2, null),
        ]),
      },
      {
        type: "gap",
        gap: { newStart: 2, newEnd: 4, oldStart: 3, count: 3 },
      },
      {
        type: "hunk",
        hunk: makeHunk([
          makeLine("context", 6, 5),
          makeLine("addition", null, 6),
        ]),
      },
    ]
    // Cursor on the addition in the second hunk
    const result = getSelectedRegion(sel({ line: 6, side: "RIGHT" }), elements)
    expect(result).toEqual({
      left: null,
      right: { start: 6, end: 6 },
    })
  })

  it("anchor pointing to nonexistent line is treated as no anchor", () => {
    const lines = [makeLine("context", 1, 1), makeLine("context", 2, 2)]
    const elements = wrapHunk(lines)
    // Anchor at line 99 doesn't exist in elements
    const result = getSelectedRegion(
      sel({ line: 2, side: "LEFT" }, { line: 99, side: "LEFT" }),
      elements,
    )
    // anchorIdx becomes null, so treated as cursor-only
    expect(result).toEqual({
      left: { start: 2, end: 2 },
      right: { start: 2, end: 2 },
    })
  })
})

// ---------------------------------------------------------------------------
// computeRegionId
// ---------------------------------------------------------------------------

describe("computeRegionId", () => {
  it("returns null when both sides are null", () => {
    const elements = wrapHunk([makeLine("context", 1, 1)])
    expect(computeRegionId({ left: null, right: null }, elements)).toBeNull()
  })

  it("returns full hunk id when both sides present", () => {
    const elements = wrapHunk([makeLine("context", 1, 1)])
    const result = computeRegionId(
      { left: { start: 5, end: 8 }, right: { start: 5, end: 10 } },
      elements,
    )
    expect(result).toEqual({
      oldStart: 5,
      oldLines: 4,
      newStart: 5,
      newLines: 6,
    })
  })

  it("returns left-only hunk id with zero new lines", () => {
    const elements = wrapHunk([makeLine("deletion", 1, null)])
    const result = computeRegionId(
      { left: { start: 3, end: 5 }, right: null },
      elements,
    )
    expect(result).toEqual({
      oldStart: 3,
      oldLines: 3,
      newStart: 0,
      newLines: 0,
    })
  })

  it("returns right-only hunk id and scans backward for oldStart", () => {
    const lines = [
      makeLine("context", 10, 20),
      makeLine("addition", null, 21),
      makeLine("addition", null, 22),
    ]
    const elements = wrapHunk(lines)
    const result = computeRegionId(
      { left: null, right: { start: 21, end: 22 } },
      elements,
    )
    expect(result).toEqual({
      oldStart: 10,
      oldLines: 0,
      newStart: 21,
      newLines: 2,
    })
  })

  it("right-only with no preceding left line gives oldStart 0", () => {
    const lines = [makeLine("addition", null, 1), makeLine("addition", null, 2)]
    const elements = wrapHunk(lines)
    const result = computeRegionId(
      { left: null, right: { start: 1, end: 2 } },
      elements,
    )
    expect(result).toEqual({
      oldStart: 0,
      oldLines: 0,
      newStart: 1,
      newLines: 2,
    })
  })

  it("single-line selection on each side", () => {
    const elements = wrapHunk([makeLine("context", 1, 1)])
    const result = computeRegionId(
      { left: { start: 7, end: 7 }, right: { start: 7, end: 7 } },
      elements,
    )
    expect(result).toEqual({
      oldStart: 7,
      oldLines: 1,
      newStart: 7,
      newLines: 1,
    })
  })

  it("right-only scans past deletions to find oldStart", () => {
    const lines = [
      makeLine("context", 5, 5),
      makeLine("deletion", 6, null),
      makeLine("deletion", 7, null),
      makeLine("addition", null, 6),
      makeLine("addition", null, 7),
    ]
    const elements = wrapHunk(lines)
    const result = computeRegionId(
      { left: null, right: { start: 6, end: 7 } },
      elements,
    )
    // Should scan backward from the addition at newLineno=6, find deletion at
    // oldLineno=7, then 6, then context at oldLineno=5. The first left-type
    // line found scanning backward is deletion at oldLineno=7.
    expect(result).toEqual({
      oldStart: 7,
      oldLines: 0,
      newStart: 6,
      newLines: 2,
    })
  })
})
