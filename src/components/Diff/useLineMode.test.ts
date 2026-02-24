import { describe, expect, it } from "vitest"

import { DiffHunk, DiffLine } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import { resolveGlobalRangeToRegion } from "./useLineMode"

function ctx(old: number, new_: number): DiffLine {
  return { lineType: "context", oldLineno: old, newLineno: new_, tokens: [] }
}

function add(new_: number): DiffLine {
  return { lineType: "addition", oldLineno: null, newLineno: new_, tokens: [] }
}

function del(old: number): DiffLine {
  return { lineType: "deletion", oldLineno: old, newLineno: null, tokens: [] }
}

/** Deletion paired with an addition (word-diff sets both line numbers). */
function pairedDel(old: number, new_: number): DiffLine {
  return { lineType: "deletion", oldLineno: old, newLineno: new_, tokens: [] }
}

/** Addition paired with a deletion (word-diff sets both line numbers). */
function pairedAdd(new_: number, old: number): DiffLine {
  return { lineType: "addition", oldLineno: old, newLineno: new_, tokens: [] }
}

function hunk(
  oldStart: number,
  oldLines: number,
  newStart: number,
  newLines: number,
  lines: DiffLine[],
): DiffHunk {
  return { oldStart, oldLines, newStart, newLines, header: "", lines }
}

function makeElements(hunks: DiffHunk[]): DiffElement[] {
  return hunks.map((h) => ({ type: "hunk", hunk: h }))
}

describe("resolveGlobalRangeToRegion (unified)", () => {
  // In unified mode the global index is simply the line index within hunk
  // lines (gaps are skipped since we only pass hunk elements).

  it("returns null for pure context selection", () => {
    // lines: 0=ctx(1,1), 1=del(2), 2=add(2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const result = resolveGlobalRangeToRegion(0, 0, elements, "unified")
    expect(result).toBeNull()
  })

  it("returns null when selection spans only context lines", () => {
    // Two context lines
    const elements = makeElements([hunk(1, 2, 1, 2, [ctx(1, 1), ctx(2, 2)])])
    const result = resolveGlobalRangeToRegion(0, 1, elements, "unified")
    expect(result).toBeNull()
  })

  it("cursor on single deletion", () => {
    // lines: 0=ctx(1,1), 1=del(2), 2=add(2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const result = resolveGlobalRangeToRegion(1, 1, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 1, // lastNewBefore from ctx(1,1)
      newLines: 0,
    })
  })

  it("cursor on single addition", () => {
    // lines: 0=ctx(1,1), 1=del(2), 2=add(2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const result = resolveGlobalRangeToRegion(2, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 2, // lastOldBefore from del(2)
      oldLines: 0,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning modification (del + add)", () => {
    // lines: 0=ctx(1,1), 1=del(2), 2=add(2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const result = resolveGlobalRangeToRegion(1, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning multiple additions", () => {
    // lines: 0=ctx(1,1), 1=add(2), 2=add(3), 3=ctx(2,4)
    const elements = makeElements([
      hunk(1, 2, 1, 4, [ctx(1, 1), add(2), add(3), ctx(2, 4)]),
    ])
    const result = resolveGlobalRangeToRegion(1, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 1, // lastOldBefore from ctx(1,1)
      oldLines: 0,
      newStart: 2,
      newLines: 2,
    })
  })

  it("pure addition after deletion uses lastOldBefore", () => {
    // Old: line1, DELETED, line3, line4
    // New: line1, line3, NEW, line4
    // lines: 0=ctx(1,1), 1=del(2), 2=ctx(3,2), 3=add(3), 4=ctx(4,4)
    const elements = makeElements([
      hunk(1, 4, 1, 4, [ctx(1, 1), del(2), ctx(3, 2), add(3), ctx(4, 4)]),
    ])
    const result = resolveGlobalRangeToRegion(3, 3, elements, "unified")
    expect(result).toEqual({
      oldStart: 3, // lastOldBefore = 3 from ctx(3,2), NOT newLineno-1
      oldLines: 0,
      newStart: 3,
      newLines: 1,
    })
  })

  it("selection across two hunks", () => {
    // Hunk1 lines: 0=ctx(1,1), 1=del(2), 2=ctx(3,2)
    // Hunk2 lines: 3=ctx(8,7), 4=add(8), 5=ctx(9,9)
    const elements = makeElements([
      hunk(1, 3, 1, 2, [ctx(1, 1), del(2), ctx(3, 2)]),
      hunk(8, 2, 7, 3, [ctx(8, 7), add(8), ctx(9, 9)]),
    ])
    const result = resolveGlobalRangeToRegion(1, 4, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 7, // del(2), ctx(3), ctx(8) → span 2..8
      newStart: 2,
      newLines: 7, // ctx(2), ctx(7), add(8) → span 2..8
    })
  })

  it("selection including context expands old and new ranges", () => {
    // lines: 0=ctx(1,1), 1=del(2), 2=add(2), 3=add(3), 4=ctx(3,4)
    const elements = makeElements([
      hunk(1, 3, 1, 4, [ctx(1, 1), del(2), add(2), add(3), ctx(3, 4)]),
    ])
    // Select ctx(1,1) through add(3)
    const result = resolveGlobalRangeToRegion(0, 3, elements, "unified")
    expect(result).toEqual({
      oldStart: 1, // from ctx(1,1)
      oldLines: 2, // ctx(old=1) + del(old=2) → span 1..2
      newStart: 1, // from ctx(1,1)
      newLines: 3, // ctx(new=1), add(new=2), add(new=3) → span 1..3
    })
  })
})

describe("resolveGlobalRangeToRegion — word-diff pairing (unified)", () => {
  it("paired deletion ignores newLineno", () => {
    // lines: 0=ctx(1,1), 1=pairedDel(2,2), 2=pairedAdd(2,2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    // Cursor on paired deletion only
    const result = resolveGlobalRangeToRegion(1, 1, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      // newLineno on the deletion must be ignored
      newStart: 1, // lastNewBefore from ctx(1,1)
      newLines: 0,
    })
  })

  it("paired addition ignores oldLineno", () => {
    // lines: 0=ctx(1,1), 1=pairedDel(2,2), 2=pairedAdd(2,2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    // Cursor on paired addition only
    const result = resolveGlobalRangeToRegion(2, 2, elements, "unified")
    expect(result).toEqual({
      // oldLineno on the addition must be ignored
      oldStart: 2, // lastOldBefore from pairedDel(2,2)
      oldLines: 0,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning paired del+add counts both sides once", () => {
    // lines: 0=ctx(1,1), 1=pairedDel(2,2), 2=pairedAdd(2,2), 3=ctx(3,3)
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    const result = resolveGlobalRangeToRegion(1, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 2,
      newLines: 1,
    })
  })
})

describe("resolveGlobalRangeToRegion — fallback tracking respects lineType", () => {
  it("lastOldBefore is not set from addition lines", () => {
    // lines: 0=add(1), 1=add(2), 2=del(1), 3=ctx(2,3)
    // Cursor on del(1). lastOldBefore should be null (0 fallback),
    // NOT 1 from add(1) or 2 from add(2) which have no old_lineno anyway
    // but if they had paired old_lineno, it should still be ignored.
    const elements = makeElements([
      hunk(1, 2, 1, 3, [pairedAdd(1, 1), pairedAdd(2, 2), del(1), ctx(2, 3)]),
    ])
    const result = resolveGlobalRangeToRegion(2, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 1,
      oldLines: 1,
      // lastNewBefore should come from the additions before, which ARE new-side
      newStart: 2, // lastNewBefore from pairedAdd(2,2)
      newLines: 0,
    })
  })

  it("lastNewBefore is not set from deletion lines", () => {
    // lines: 0=pairedDel(1,1), 1=pairedDel(2,2), 2=add(1), 3=ctx(3,2)
    // Cursor on add(1). lastNewBefore should be null (0 fallback),
    // NOT from pairedDel's newLineno.
    const elements = makeElements([
      hunk(1, 3, 1, 2, [pairedDel(1, 1), pairedDel(2, 2), add(1), ctx(3, 2)]),
    ])
    const result = resolveGlobalRangeToRegion(2, 2, elements, "unified")
    expect(result).toEqual({
      oldStart: 2, // lastOldBefore from pairedDel(2,2) which IS old-side
      oldLines: 0,
      newStart: 1,
      newLines: 1,
    })
  })
})
