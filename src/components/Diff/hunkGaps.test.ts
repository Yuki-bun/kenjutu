import { describe, expect, it } from "vitest"

import { DiffHunk, DiffLine } from "@/bindings"

import { augmentHunks, buildDiffElements, DiffElement } from "./hunkGaps"

function makeLine(newLineno: number, oldLineno: number): DiffLine {
  return {
    lineType: "context",
    newLineno,
    oldLineno,
    tokens: [],
  }
}

function makeHunk(
  oldStart: number,
  oldLines: number,
  newStart: number,
  newLines: number,
  lines?: DiffLine[],
): DiffHunk {
  return {
    oldStart,
    oldLines,
    newStart,
    newLines,
    header: "",
    lines: lines ?? [],
  }
}

function gaps(elements: DiffElement[]) {
  return elements.filter((e) => e.type === "gap").map((e) => e.gap)
}

function hunks(elements: DiffElement[]) {
  return elements.filter((e) => e.type === "hunk").map((e) => e.hunk)
}

describe("buildDiffElements", () => {
  it("returns empty array for empty hunks", () => {
    expect(buildDiffElements([], 100)).toEqual([])
  })

  it("excludes zero-count gaps", () => {
    // Hunk covers entire file → no gaps
    const elements = buildDiffElements([makeHunk(1, 5, 1, 5)], 5)
    expect(gaps(elements)).toHaveLength(0)
    expect(hunks(elements)).toHaveLength(1)
  })

  it("interleaves gap-hunk-gap for a single hunk in the middle", () => {
    const elements = buildDiffElements([makeHunk(4, 3, 4, 3)], 10)

    expect(elements).toHaveLength(3)
    expect(elements[0].type).toBe("gap")
    expect(elements[1].type).toBe("hunk")
    expect(elements[2].type).toBe("gap")

    const [before, after] = gaps(elements)
    expect(before).toEqual({ newStart: 1, newEnd: 3, oldStart: 1, count: 3 })
    expect(after).toEqual({ newStart: 7, newEnd: 10, oldStart: 7, count: 4 })
  })

  it("omits leading gap when hunk starts at line 1", () => {
    const elements = buildDiffElements([makeHunk(1, 3, 1, 3)], 10)

    expect(elements).toHaveLength(2)
    expect(elements[0].type).toBe("hunk")
    expect(elements[1].type).toBe("gap")

    expect(gaps(elements)[0]).toEqual({
      newStart: 4,
      newEnd: 10,
      oldStart: 4,
      count: 7,
    })
  })

  it("interleaves correctly for two hunks with a gap between", () => {
    const elements = buildDiffElements(
      [makeHunk(2, 3, 2, 3), makeHunk(8, 2, 8, 2)],
      12,
    )

    // gap, hunk, gap, hunk, gap
    expect(elements.map((e) => e.type)).toEqual([
      "gap",
      "hunk",
      "gap",
      "hunk",
      "gap",
    ])

    const g = gaps(elements)
    expect(g[0]).toEqual({ newStart: 1, newEnd: 1, oldStart: 1, count: 1 })
    expect(g[1]).toEqual({ newStart: 5, newEnd: 7, oldStart: 5, count: 3 })
    expect(g[2]).toEqual({ newStart: 10, newEnd: 12, oldStart: 10, count: 3 })
  })

  it("omits between-gap when hunks are adjacent", () => {
    // Hunk 1: lines 1-3, Hunk 2: lines 4-6, file=8
    const elements = buildDiffElements(
      [makeHunk(1, 3, 1, 3), makeHunk(4, 3, 4, 3)],
      8,
    )

    // hunk, hunk, gap (no gap between, no leading gap)
    expect(elements.map((e) => e.type)).toEqual(["hunk", "hunk", "gap"])
    expect(gaps(elements)[0]).toEqual({
      newStart: 7,
      newEnd: 8,
      oldStart: 7,
      count: 2,
    })
  })
})

describe("augmentHunks", () => {
  it("returns hunks unchanged when fetchedLines is empty", () => {
    const hunks = [
      makeHunk(1, 3, 1, 3, [makeLine(1, 1), makeLine(2, 2), makeLine(3, 3)]),
    ]
    const result = augmentHunks(hunks, new Map(), 10)
    expect(result).toEqual(hunks)
  })

  it("returns hunks unchanged when hunks is empty", () => {
    const fetched = new Map([[1, makeLine(1, 1)]])
    expect(augmentHunks([], fetched, 10)).toEqual([])
  })

  it("prepends context lines before the first hunk", () => {
    const hunks = [makeHunk(4, 2, 4, 2, [makeLine(4, 4), makeLine(5, 5)])]
    const fetched = new Map([
      [2, makeLine(2, 2)],
      [3, makeLine(3, 3)],
    ])

    const result = augmentHunks(hunks, fetched, 10)

    expect(result).toHaveLength(1)
    expect(result[0].newStart).toBe(2)
    expect(result[0].oldStart).toBe(2)
    expect(result[0].newLines).toBe(4)
    expect(result[0].lines).toHaveLength(4)
    expect(result[0].lines[0].newLineno).toBe(2)
    expect(result[0].lines[1].newLineno).toBe(3)
  })

  it("appends context lines after the last hunk", () => {
    const hunks = [
      makeHunk(1, 3, 1, 3, [makeLine(1, 1), makeLine(2, 2), makeLine(3, 3)]),
    ]
    const fetched = new Map([
      [4, makeLine(4, 4)],
      [5, makeLine(5, 5)],
    ])

    const result = augmentHunks(hunks, fetched, 6)

    expect(result).toHaveLength(1)
    expect(result[0].newLines).toBe(5)
    expect(result[0].lines).toHaveLength(5)
    expect(result[0].lines[3].newLineno).toBe(4)
    expect(result[0].lines[4].newLineno).toBe(5)
  })

  it("expands from previous hunk side (bottom lines)", () => {
    const hunks = [
      makeHunk(1, 3, 1, 3, [makeLine(1, 1), makeLine(2, 2), makeLine(3, 3)]),
      makeHunk(8, 3, 8, 3, [makeLine(8, 8), makeLine(9, 9), makeLine(10, 10)]),
    ]
    const fetched = new Map([
      [4, makeLine(4, 4)],
      [5, makeLine(5, 5)],
    ])

    const result = augmentHunks(hunks, fetched, 10)

    expect(result[0].newLines).toBe(5)
    expect(result[0].lines).toHaveLength(5)
    expect(result[0].lines[3].newLineno).toBe(4)
    expect(result[0].lines[4].newLineno).toBe(5)
    expect(result[1].newStart).toBe(8)
  })

  it("expands from next hunk side (top lines)", () => {
    const hunks = [
      makeHunk(1, 3, 1, 3, [makeLine(1, 1), makeLine(2, 2), makeLine(3, 3)]),
      makeHunk(8, 3, 8, 3, [makeLine(8, 8), makeLine(9, 9), makeLine(10, 10)]),
    ]
    const fetched = new Map([
      [6, makeLine(6, 6)],
      [7, makeLine(7, 7)],
    ])

    const result = augmentHunks(hunks, fetched, 10)

    expect(result[0].newLines).toBe(3)
    expect(result[1].newStart).toBe(6)
    expect(result[1].oldStart).toBe(6)
    expect(result[1].newLines).toBe(5)
    expect(result[1].lines).toHaveLength(5)
    expect(result[1].lines[0].newLineno).toBe(6)
  })

  it("merges hunks when gap is fully filled", () => {
    const hunks = [
      makeHunk(1, 3, 1, 3, [makeLine(1, 1), makeLine(2, 2), makeLine(3, 3)]),
      makeHunk(6, 3, 6, 3, [makeLine(6, 6), makeLine(7, 7), makeLine(8, 8)]),
    ]
    const fetched = new Map([
      [4, makeLine(4, 4)],
      [5, makeLine(5, 5)],
    ])

    const result = augmentHunks(hunks, fetched, 10)

    expect(result).toHaveLength(1)
    expect(result[0].newStart).toBe(1)
    expect(result[0].newLines).toBe(8)
    expect(result[0].lines).toHaveLength(8)
  })
})
