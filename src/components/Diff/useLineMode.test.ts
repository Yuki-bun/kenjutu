import { describe, expect, it } from "vitest"

import { DiffHunk, DiffLine } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import {
  LineIdentity,
  lineIdentityForDiffLine,
  resolveSelectionToRegion,
} from "./useLineMode"

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

/** Helper to get the identity for a line at a given position in elements. */
function identityAt(
  elements: DiffElement[],
  hunkIndex: number,
  lineIndex: number,
): LineIdentity {
  const el = elements[hunkIndex]
  if (el.type !== "hunk") throw new Error("Not a hunk")
  const id = lineIdentityForDiffLine(el.hunk.lines[lineIndex])
  if (!id) throw new Error("No identity for line")
  return id
}

describe("resolveSelectionToRegion (unified)", () => {
  it("returns null for pure context selection", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const id = identityAt(elements, 0, 0) // ctx(1,1) → RIGHT:1
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toBeNull()
  })

  it("returns null when selection spans only context lines", () => {
    const elements = makeElements([hunk(1, 2, 1, 2, [ctx(1, 1), ctx(2, 2)])])
    const start = identityAt(elements, 0, 0) // ctx(1,1) → RIGHT:1
    const end = identityAt(elements, 0, 1) // ctx(2,2) → RIGHT:2
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toBeNull()
  })

  it("cursor on single deletion", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const id = identityAt(elements, 0, 1) // del(2) → LEFT:2
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 1, // lastNewBefore from ctx(1,1)
      newLines: 0,
    })
  })

  it("cursor on single addition", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const id = identityAt(elements, 0, 2) // add(2) → RIGHT:2
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 2, // lastOldBefore from del(2)
      oldLines: 0,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning modification (del + add)", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [ctx(1, 1), del(2), add(2), ctx(3, 3)]),
    ])
    const start = identityAt(elements, 0, 1) // del(2) → LEFT:2
    const end = identityAt(elements, 0, 2) // add(2) → RIGHT:2
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning multiple additions", () => {
    const elements = makeElements([
      hunk(1, 2, 1, 4, [ctx(1, 1), add(2), add(3), ctx(2, 4)]),
    ])
    const start = identityAt(elements, 0, 1) // add(2) → RIGHT:2
    const end = identityAt(elements, 0, 2) // add(3) → RIGHT:3
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toEqual({
      oldStart: 1, // lastOldBefore from ctx(1,1)
      oldLines: 0,
      newStart: 2,
      newLines: 2,
    })
  })

  it("pure addition after deletion uses lastOldBefore", () => {
    const elements = makeElements([
      hunk(1, 4, 1, 4, [ctx(1, 1), del(2), ctx(3, 2), add(3), ctx(4, 4)]),
    ])
    const id = identityAt(elements, 0, 3) // add(3) → RIGHT:3
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 3, // lastOldBefore = 3 from ctx(3,2), NOT newLineno-1
      oldLines: 0,
      newStart: 3,
      newLines: 1,
    })
  })

  it("selection across two hunks", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 2, [ctx(1, 1), del(2), ctx(3, 2)]),
      hunk(8, 2, 7, 3, [ctx(8, 7), add(8), ctx(9, 9)]),
    ])
    const start = identityAt(elements, 0, 1) // del(2) → LEFT:2
    const end = identityAt(elements, 1, 1) // add(8) → RIGHT:8
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 7, // del(2), ctx(3), ctx(8) → span 2..8
      newStart: 2,
      newLines: 7, // ctx(2), ctx(7), add(8) → span 2..8
    })
  })

  it("selection including context expands old and new ranges", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 4, [ctx(1, 1), del(2), add(2), add(3), ctx(3, 4)]),
    ])
    const start = identityAt(elements, 0, 0) // ctx(1,1) → RIGHT:1
    const end = identityAt(elements, 0, 3) // add(3) → RIGHT:3
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toEqual({
      oldStart: 1, // from ctx(1,1)
      oldLines: 2, // ctx(old=1) + del(old=2) → span 1..2
      newStart: 1, // from ctx(1,1)
      newLines: 3, // ctx(new=1), add(new=2), add(new=3) → span 1..3
    })
  })
})

describe("resolveSelectionToRegion — word-diff pairing (unified)", () => {
  it("paired deletion ignores newLineno", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    const id = identityAt(elements, 0, 1) // pairedDel(2,2) → LEFT:2
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 1, // lastNewBefore from ctx(1,1)
      newLines: 0,
    })
  })

  it("paired addition ignores oldLineno", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    const id = identityAt(elements, 0, 2) // pairedAdd(2,2) → RIGHT:2
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 2, // lastOldBefore from pairedDel(2,2)
      oldLines: 0,
      newStart: 2,
      newLines: 1,
    })
  })

  it("selection spanning paired del+add counts both sides once", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 3, [
        ctx(1, 1),
        pairedDel(2, 2),
        pairedAdd(2, 2),
        ctx(3, 3),
      ]),
    ])
    const start = identityAt(elements, 0, 1) // pairedDel(2,2) → LEFT:2
    const end = identityAt(elements, 0, 2) // pairedAdd(2,2) → RIGHT:2
    const result = resolveSelectionToRegion(start, end, elements, "unified")
    expect(result).toEqual({
      oldStart: 2,
      oldLines: 1,
      newStart: 2,
      newLines: 1,
    })
  })
})

describe("resolveSelectionToRegion — fallback tracking respects lineType", () => {
  it("lastOldBefore is not set from addition lines", () => {
    const elements = makeElements([
      hunk(1, 2, 1, 3, [pairedAdd(1, 1), pairedAdd(2, 2), del(1), ctx(2, 3)]),
    ])
    const id = identityAt(elements, 0, 2) // del(1) → LEFT:1
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 1,
      oldLines: 1,
      newStart: 2, // lastNewBefore from pairedAdd(2,2)
      newLines: 0,
    })
  })

  it("lastNewBefore is not set from deletion lines", () => {
    const elements = makeElements([
      hunk(1, 3, 1, 2, [pairedDel(1, 1), pairedDel(2, 2), add(1), ctx(3, 2)]),
    ])
    const id = identityAt(elements, 0, 2) // add(1) → RIGHT:1
    const result = resolveSelectionToRegion(id, id, elements, "unified")
    expect(result).toEqual({
      oldStart: 2, // lastOldBefore from pairedDel(2,2) which IS old-side
      oldLines: 0,
      newStart: 1,
      newLines: 1,
    })
  })
})

describe("lineIdentityForDiffLine", () => {
  it("deletion uses oldLineno and LEFT side", () => {
    const id = lineIdentityForDiffLine(del(5))
    expect(id).toEqual({ line: 5, side: "LEFT" })
  })

  it("addition uses newLineno and RIGHT side", () => {
    const id = lineIdentityForDiffLine(add(3))
    expect(id).toEqual({ line: 3, side: "RIGHT" })
  })

  it("context uses newLineno and RIGHT side", () => {
    const id = lineIdentityForDiffLine(ctx(10, 12))
    expect(id).toEqual({ line: 12, side: "RIGHT" })
  })

  it("returns null for line with no line numbers", () => {
    const line: DiffLine = {
      lineType: "addeofnl",
      oldLineno: null,
      newLineno: null,
      tokens: [],
    }
    expect(lineIdentityForDiffLine(line)).toBeNull()
  })
})
