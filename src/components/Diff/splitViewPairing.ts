import { DiffLine } from "@/bindings"

export type PairedLine = {
  left: DiffLine | null
  right: DiffLine | null
}

type ProcessResult = {
  pairs: PairedLine[]
  nextIndex: number
}

export function pairLinesForSplitView(lines: DiffLine[]): PairedLine[] {
  return processLines(lines, 0).pairs
}

function processLines(lines: DiffLine[], startIndex: number): ProcessResult {
  if (startIndex >= lines.length) {
    return { pairs: [], nextIndex: startIndex }
  }

  const line = lines[startIndex]

  if (
    line.lineType === "context" ||
    line.lineType === "addeofnl" ||
    line.lineType === "deleofnl"
  ) {
    const contextPair: PairedLine = { left: line, right: line }
    const rest = processLines(lines, startIndex + 1)
    return { pairs: [contextPair, ...rest.pairs], nextIndex: rest.nextIndex }
  }

  if (line.lineType === "deletion") {
    const deletionsResult = collectDeletions(lines, startIndex)
    const additionsResult = collectAdditions(lines, deletionsResult.nextIndex)
    const alignedPairs = alignChangedBlock(
      deletionsResult.lines,
      additionsResult.lines,
    )
    const rest = processLines(lines, additionsResult.nextIndex)
    return {
      pairs: [...alignedPairs, ...rest.pairs],
      nextIndex: rest.nextIndex,
    }
  }

  const additionPair: PairedLine = { left: null, right: line }
  const rest = processLines(lines, startIndex + 1)
  return { pairs: [additionPair, ...rest.pairs], nextIndex: rest.nextIndex }
}

function collectDeletions(
  lines: DiffLine[],
  startIndex: number,
): { lines: DiffLine[]; nextIndex: number } {
  const endIndex = lines
    .slice(startIndex)
    .findIndex((line) => line.lineType !== "deletion")

  const actualEndIndex = endIndex === -1 ? lines.length : startIndex + endIndex

  return {
    lines: lines.slice(startIndex, actualEndIndex),
    nextIndex: actualEndIndex,
  }
}

function collectAdditions(
  lines: DiffLine[],
  startIndex: number,
): { lines: DiffLine[]; nextIndex: number } {
  const endIndex = lines
    .slice(startIndex)
    .findIndex((line) => line.lineType !== "addition")

  const actualEndIndex = endIndex === -1 ? lines.length : startIndex + endIndex

  return {
    lines: lines.slice(startIndex, actualEndIndex),
    nextIndex: actualEndIndex,
  }
}

/** Align a block of consecutive deletions + additions using backend match info. */
function alignChangedBlock(
  deletions: DiffLine[],
  additions: DiffLine[],
): PairedLine[] {
  const addByNewLineno = buildAdditionMap(additions)
  const matchPairs = findMatchPairs(deletions, addByNewLineno)

  if (matchPairs.length === 0) {
    return positionalPairing(deletions, additions)
  }

  return alignWithMatchPairs(deletions, additions, matchPairs)
}

function buildAdditionMap(additions: DiffLine[]): Map<number, number> {
  return additions.reduce((map, addition, idx) => {
    if (addition.newLineno != null) {
      map.set(addition.newLineno, idx)
    }
    return map
  }, new Map<number, number>())
}

function findMatchPairs(
  deletions: DiffLine[],
  addByNewLineno: Map<number, number>,
): [number, number][] {
  return deletions.reduce<[number, number][]>((pairs, deletion, delIdx) => {
    if (deletion.newLineno != null) {
      const addIdx = addByNewLineno.get(deletion.newLineno)
      if (addIdx != null) {
        return [...pairs, [delIdx, addIdx]]
      }
    }
    return pairs
  }, [])
}

function positionalPairing(
  deletions: DiffLine[],
  additions: DiffLine[],
): PairedLine[] {
  const maxLen = Math.max(deletions.length, additions.length)
  return Array.from({ length: maxLen }, (_, j) => ({
    left: deletions[j] ?? null,
    right: additions[j] ?? null,
  }))
}

function alignWithMatchPairs(
  deletions: DiffLine[],
  additions: DiffLine[],
  matchPairs: [number, number][],
): PairedLine[] {
  type AlignState = {
    pairs: PairedLine[]
    delPtr: number
    addPtr: number
  }

  const finalState = matchPairs.reduce<AlignState>(
    (state, [delIdx, addIdx]) => {
      // Emit unmatched deletions before this pair
      const unmatchedDels = createRange(state.delPtr, delIdx).map((i) => ({
        left: deletions[i],
        right: null,
      }))

      // Emit unmatched additions before this pair
      const unmatchedAdds = createRange(state.addPtr, addIdx).map((i) => ({
        left: null,
        right: additions[i],
      }))

      // Emit the matched pair
      const matchedPair: PairedLine = {
        left: deletions[delIdx],
        right: additions[addIdx],
      }

      return {
        pairs: [
          ...state.pairs,
          ...unmatchedDels,
          ...unmatchedAdds,
          matchedPair,
        ],
        delPtr: delIdx + 1,
        addPtr: addIdx + 1,
      }
    },
    { pairs: [], delPtr: 0, addPtr: 0 },
  )

  // Remaining unmatched lines after all pairs
  const remainingDels = createRange(finalState.delPtr, deletions.length).map(
    (i) => ({
      left: deletions[i],
      right: null,
    }),
  )

  const remainingAdds = createRange(finalState.addPtr, additions.length).map(
    (i) => ({
      left: null,
      right: additions[i],
    }),
  )

  return [...finalState.pairs, ...remainingDels, ...remainingAdds]
}

function createRange(start: number, end: number): number[] {
  return Array.from({ length: Math.max(0, end - start) }, (_, i) => start + i)
}
