import { DiffHunk, DiffLine } from "@/bindings"

export type HunkGap = {
  /** 1-based first hidden new-file line */
  newStart: number
  /** 1-based last hidden new-file line */
  newEnd: number
  /** 1-based first hidden old-file line */
  oldStart: number
  /** Number of hidden lines */
  count: number
}

export type DiffElement =
  | { type: "gap"; gap: HunkGap }
  | { type: "hunk"; hunk: DiffHunk }

/**
 * Build an interleaved sequence of gaps and hunks.
 * Gaps with count=0 are excluded.
 */
export function buildDiffElements(
  hunks: DiffHunk[],
  newFileLines: number,
): DiffElement[] {
  if (hunks.length === 0) return []

  const elements: DiffElement[] = []

  // Gap before first hunk
  const firstHunk = hunks[0]
  const beforeNewEnd = firstHunk.newStart - 1
  const beforeCount = Math.max(0, beforeNewEnd)
  if (beforeCount > 0) {
    elements.push({
      type: "gap",
      gap: {
        newStart: 1,
        newEnd: beforeNewEnd,
        oldStart: 1,
        count: beforeCount,
      },
    })
  }

  for (let i = 0; i < hunks.length; i++) {
    elements.push({ type: "hunk", hunk: hunks[i] })

    if (i < hunks.length - 1) {
      const prev = hunks[i]
      const next = hunks[i + 1]
      const prevNewEnd = prev.newStart + prev.newLines - 1
      const prevOldEnd = prev.oldStart + prev.oldLines - 1
      const gapNewStart = prevNewEnd + 1
      const gapNewEnd = next.newStart - 1
      const gapOldStart = prevOldEnd + 1
      const count = Math.max(0, gapNewEnd - gapNewStart + 1)

      if (count > 0) {
        elements.push({
          type: "gap",
          gap: {
            newStart: gapNewStart,
            newEnd: gapNewEnd,
            oldStart: gapOldStart,
            count,
          },
        })
      }
    }
  }

  // Gap after last hunk
  const lastHunk = hunks[hunks.length - 1]
  const afterNewStart = lastHunk.newStart + lastHunk.newLines
  const afterOldStart = lastHunk.oldStart + lastHunk.oldLines
  const trailingCount = Math.max(0, newFileLines - afterNewStart + 1)
  if (trailingCount > 0) {
    elements.push({
      type: "gap",
      gap: {
        newStart: afterNewStart,
        newEnd: newFileLines,
        oldStart: afterOldStart,
        count: trailingCount,
      },
    })
  }

  return elements
}

/**
 * Attach fetched context lines to hunks, producing new augmented hunks.
 * Context lines are merged into the nearest adjacent hunk, and if a gap
 * between two hunks is fully filled, the hunks are merged.
 */
export function augmentHunks(
  hunks: DiffHunk[],
  fetchedLines: Map<number, DiffLine>,
  newFileLines: number,
): DiffHunk[] {
  if (hunks.length === 0 || fetchedLines.size === 0) return hunks

  // Build a working copy of hunks
  const result: DiffHunk[] = hunks.map((h) => ({
    ...h,
    lines: [...h.lines],
  }))

  // Compute gap ranges from original hunks
  const gapRanges: { newStart: number; newEnd: number }[] = []
  gapRanges.push({ newStart: 1, newEnd: hunks[0].newStart - 1 })
  for (let i = 0; i < hunks.length - 1; i++) {
    const prev = hunks[i]
    const next = hunks[i + 1]
    gapRanges.push({
      newStart: prev.newStart + prev.newLines,
      newEnd: next.newStart - 1,
    })
  }
  const last = hunks[hunks.length - 1]
  gapRanges.push({
    newStart: last.newStart + last.newLines,
    newEnd: newFileLines,
  })

  // Collect fetched lines per gap and attach to adjacent hunks
  for (let gapIdx = 0; gapIdx < gapRanges.length; gapIdx++) {
    const { newStart, newEnd } = gapRanges[gapIdx]
    const linesInGap: DiffLine[] = []
    for (let n = newStart; n <= newEnd; n++) {
      const line = fetchedLines.get(n)
      if (line) linesInGap.push(line)
    }
    if (linesInGap.length === 0) continue

    if (gapIdx === 0) {
      // Before first hunk — prepend to first hunk
      const hunk = result[0]
      hunk.lines = [...linesInGap, ...hunk.lines]
      hunk.newStart = linesInGap[0].newLineno!
      hunk.oldStart = linesInGap[0].oldLineno!
      hunk.newLines += linesInGap.length
      hunk.oldLines += linesInGap.length
    } else if (gapIdx === gapRanges.length - 1) {
      // After last hunk — append to last hunk
      const hunk = result[result.length - 1]
      hunk.lines = [...hunk.lines, ...linesInGap]
      hunk.newLines += linesInGap.length
      hunk.oldLines += linesInGap.length
    } else {
      // Between hunk[gapIdx-1] and hunk[gapIdx]
      // Split fetched lines into those contiguous with previous vs next hunk
      const prevHunk = result[gapIdx - 1]
      const nextHunk = result[gapIdx]

      // Find contiguous block from gap start (touches previous hunk)
      const bottomLines: DiffLine[] = []
      for (let n = newStart; n <= newEnd; n++) {
        if (!fetchedLines.has(n)) break
        bottomLines.push(fetchedLines.get(n)!)
      }

      // Find contiguous block from gap end (touches next hunk)
      const topLines: DiffLine[] = []
      for (let n = newEnd; n >= newStart; n--) {
        if (!fetchedLines.has(n)) break
        topLines.unshift(fetchedLines.get(n)!)
      }

      const bottomEnd =
        bottomLines.length > 0
          ? bottomLines[bottomLines.length - 1].newLineno!
          : newStart - 1
      const topStart = topLines.length > 0 ? topLines[0].newLineno! : newEnd + 1

      if (bottomEnd >= topStart) {
        // Blocks overlap — all lines form one contiguous range, append to prev
        prevHunk.lines = [...prevHunk.lines, ...linesInGap]
        prevHunk.newLines += linesInGap.length
        prevHunk.oldLines += linesInGap.length
      } else {
        if (bottomLines.length > 0) {
          prevHunk.lines = [...prevHunk.lines, ...bottomLines]
          prevHunk.newLines += bottomLines.length
          prevHunk.oldLines += bottomLines.length
        }
        if (topLines.length > 0) {
          nextHunk.lines = [...topLines, ...nextHunk.lines]
          nextHunk.newStart = topLines[0].newLineno!
          nextHunk.oldStart = topLines[0].oldLineno!
          nextHunk.newLines += topLines.length
          nextHunk.oldLines += topLines.length
        }
      }
    }
  }

  // Merge adjacent hunks whose ranges now touch
  const merged: DiffHunk[] = [result[0]]
  for (let i = 1; i < result.length; i++) {
    const prev = merged[merged.length - 1]
    const curr = result[i]
    if (prev.newStart + prev.newLines >= curr.newStart) {
      prev.lines = [...prev.lines, ...curr.lines]
      prev.newLines = curr.newStart + curr.newLines - prev.newStart
      prev.oldLines = curr.oldStart + curr.oldLines - prev.oldStart
    } else {
      merged.push(curr)
    }
  }

  return merged
}
