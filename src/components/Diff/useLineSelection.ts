import { useMemo } from "react"

import { HunkId } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import { pairLinesForSplitView } from "./SplitDiff"
import { CommentLineState } from "./types"
import { DiffViewMode } from "./useDiffViewMode"

export type LineSelectionState = {
  cursorIndex: number
  /** null = no selection */
  anchor: number | null
}

export type SelectionRange = {
  readonly start: number
  readonly end: number
}

export type SelectionHighlightProps = {
  readonly elementRowOffsets: ReadonlyMap<number, number>
  readonly cursorIndex: number
  readonly selectionRange: SelectionRange | null
}

export type LineNavProps = {
  navIndex: number
  isCursor: boolean
  isSelected: boolean
}

export type RowLayout = {
  count: number
  offsets: ReadonlyMap<number, number>
  hunkStarts: number[]
}

export function computeRowLayout(
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): RowLayout {
  let count = 0
  const offsets = new Map<number, number>()
  const hunkStarts: number[] = []
  for (let i = 0; i < elements.length; i++) {
    const el = elements[i]
    if (el.type !== "hunk") continue
    const rowCount =
      diffViewMode === "split"
        ? pairLinesForSplitView(el.hunk.lines).length
        : el.hunk.lines.length
    offsets.set(i, count)
    hunkStarts.push(count)
    count += rowCount
  }
  return { count, offsets, hunkStarts }
}

type ResolvedLine = {
  line: number
  side: "LEFT" | "RIGHT"
}

export function resolveGlobalIndex(
  globalIndex: number,
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): ResolvedLine | null {
  let offset = 0
  for (const el of elements) {
    if (el.type !== "hunk") continue
    const pairs =
      diffViewMode === "split"
        ? pairLinesForSplitView(el.hunk.lines)
        : undefined
    const rowCount = pairs ? pairs.length : el.hunk.lines.length
    if (globalIndex < offset + rowCount) {
      const localIndex = globalIndex - offset
      if (pairs) {
        const pair = pairs[localIndex]
        if (pair.right?.newLineno != null) {
          return { line: pair.right.newLineno, side: "RIGHT" }
        }
        if (pair.left?.oldLineno != null) {
          return { line: pair.left.oldLineno, side: "LEFT" }
        }
        return null
      } else {
        const diffLine = el.hunk.lines[localIndex]
        if (diffLine.lineType === "deletion") {
          return diffLine.oldLineno != null
            ? { line: diffLine.oldLineno, side: "LEFT" }
            : null
        }
        const lineNumber = diffLine.newLineno ?? diffLine.oldLineno
        return lineNumber != null ? { line: lineNumber, side: "RIGHT" } : null
      }
    }
    offset += rowCount
  }
  return null
}

export function lineToGlobalIndex(
  line: number,
  side: "LEFT" | "RIGHT",
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): number | null {
  let offset = 0
  for (const el of elements) {
    if (el.type !== "hunk") continue
    const pairs =
      diffViewMode === "split"
        ? pairLinesForSplitView(el.hunk.lines)
        : undefined
    const rowCount = pairs ? pairs.length : el.hunk.lines.length

    if (pairs) {
      for (let i = 0; i < pairs.length; i++) {
        const pair = pairs[i]
        if (side === "LEFT" && pair.left?.oldLineno === line) {
          return offset + i
        }
        if (side === "RIGHT" && pair.right?.newLineno === line) {
          return offset + i
        }
      }
    } else {
      for (let i = 0; i < el.hunk.lines.length; i++) {
        const diffLine = el.hunk.lines[i]
        if (
          side === "LEFT" &&
          diffLine.lineType === "deletion" &&
          diffLine.oldLineno === line
        ) {
          return offset + i
        }
        if (side === "RIGHT" && diffLine.lineType !== "deletion") {
          const lineNumber = diffLine.newLineno ?? diffLine.oldLineno
          if (lineNumber === line) {
            return offset + i
          }
        }
      }
    }

    offset += rowCount
  }
  return null
}

export function resolveGlobalRangeToRegion(
  startIndex: number,
  endIndex: number,
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): HunkId | null {
  let minOld = Infinity
  let maxOld = -Infinity
  let minNew = Infinity
  let maxNew = -Infinity
  let lastOldBefore: number | null = null
  let lastNewBefore: number | null = null
  let hasChange = false

  let offset = 0
  for (const el of elements) {
    if (el.type !== "hunk") continue
    const lines = el.hunk.lines
    const pairs =
      diffViewMode === "split" ? pairLinesForSplitView(lines) : undefined
    const rowCount = pairs ? pairs.length : lines.length

    for (let localIdx = 0; localIdx < rowCount; localIdx++) {
      const gi = offset + localIdx

      if (pairs) {
        const pair = pairs[localIdx]
        if (gi < startIndex) {
          if (pair.left?.oldLineno != null) lastOldBefore = pair.left.oldLineno
          if (pair.right?.newLineno != null)
            lastNewBefore = pair.right.newLineno
        } else if (gi <= endIndex) {
          if (pair.left?.oldLineno != null) {
            minOld = Math.min(minOld, pair.left.oldLineno)
            maxOld = Math.max(maxOld, pair.left.oldLineno)
          }
          if (pair.right?.newLineno != null) {
            minNew = Math.min(minNew, pair.right.newLineno)
            maxNew = Math.max(maxNew, pair.right.newLineno)
          }
          if (
            pair.left?.lineType === "addition" ||
            pair.left?.lineType === "deletion" ||
            pair.right?.lineType === "addition" ||
            pair.right?.lineType === "deletion"
          ) {
            hasChange = true
          }
        }
      } else {
        const line = lines[localIdx]
        const isOldSide =
          line.lineType === "context" || line.lineType === "deletion"
        const isNewSide =
          line.lineType === "context" || line.lineType === "addition"

        if (gi < startIndex) {
          if (isOldSide && line.oldLineno != null)
            lastOldBefore = line.oldLineno
          if (isNewSide && line.newLineno != null)
            lastNewBefore = line.newLineno
        } else if (gi <= endIndex) {
          if (line.lineType === "addition" || line.lineType === "deletion") {
            hasChange = true
          }
          if (isOldSide && line.oldLineno != null) {
            minOld = Math.min(minOld, line.oldLineno)
            maxOld = Math.max(maxOld, line.oldLineno)
          }
          if (isNewSide && line.newLineno != null) {
            minNew = Math.min(minNew, line.newLineno)
            maxNew = Math.max(maxNew, line.newLineno)
          }
        }
      }
    }

    offset += rowCount
    if (offset > endIndex) break
  }

  if (!hasChange) return null

  const hasOld = minOld !== Infinity
  const hasNew = minNew !== Infinity

  return {
    oldStart: hasOld ? minOld : (lastOldBefore ?? 0),
    oldLines: hasOld ? maxOld - minOld + 1 : 0,
    newStart: hasNew ? minNew : (lastNewBefore ?? 0),
    newLines: hasNew ? maxNew - minNew + 1 : 0,
  }
}

export function selectionToCommentLineState(
  cursorIndex: number,
  anchor: number | null,
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): CommentLineState {
  const cursorResolved = resolveGlobalIndex(cursorIndex, elements, diffViewMode)
  if (!cursorResolved) return null

  if (anchor != null) {
    const anchorResolved = resolveGlobalIndex(anchor, elements, diffViewMode)
    if (anchorResolved && anchorResolved.side === cursorResolved.side) {
      const startLine = Math.min(anchorResolved.line, cursorResolved.line)
      const endLine = Math.max(anchorResolved.line, cursorResolved.line)
      if (startLine !== endLine) {
        return {
          line: endLine,
          side: cursorResolved.side,
          startLine,
          startSide: cursorResolved.side,
        }
      }
    }
  }

  return { line: cursorResolved.line, side: cursorResolved.side }
}

export function getLineHighlightBg({
  isCursor,
  isSelected,
  defaultBg,
}: {
  isCursor?: boolean
  isSelected?: boolean
  defaultBg: string
}): string {
  if (isCursor) return "bg-yellow-200/80 dark:bg-yellow-700/40"
  if (isSelected) return "bg-yellow-100/60 dark:bg-yellow-800/30"
  return defaultBg
}

export type LineSelectionControl = {
  state: LineSelectionState | null
  setState: React.Dispatch<React.SetStateAction<LineSelectionState | null>>
  onExit: () => void
}

export function useLineSelection({
  elements,
  diffViewMode,
  state,
  setState,
}: {
  elements: DiffElement[]
  diffViewMode: DiffViewMode
  state: LineSelectionState | null
  setState: React.Dispatch<React.SetStateAction<LineSelectionState | null>>
}) {
  const rowLayout = useMemo(
    () => computeRowLayout(elements, diffViewMode),
    [elements, diffViewMode],
  )

  const moveCursor = (index: number) => {
    setState((prev) => {
      if (!prev) return prev
      const clamped = Math.max(0, Math.min(index, rowLayout.count - 1))
      return { ...prev, cursorIndex: clamped, anchor: null }
    })
  }

  const moveCursorBy = (delta: number) => {
    setState((prev) => {
      if (!prev) return prev
      const next = Math.max(
        0,
        Math.min(prev.cursorIndex + delta, rowLayout.count - 1),
      )
      if (next === prev.cursorIndex) return prev
      return { ...prev, cursorIndex: next }
    })
  }

  const startSelect = (index: number) => {
    setState((prev) => {
      if (!prev) return prev
      const clamped = Math.max(0, Math.min(index, rowLayout.count - 1))
      return { cursorIndex: clamped, anchor: clamped }
    })
  }

  const selectTo = (index: number) => {
    setState((prev) => {
      if (!prev) return prev
      const clamped = Math.max(0, Math.min(index, rowLayout.count - 1))
      return { ...prev, cursorIndex: clamped }
    })
  }

  const toggleSelect = () => {
    setState((prev) => {
      if (!prev) return prev
      if (prev.anchor != null) {
        return { ...prev, anchor: null }
      }
      return { ...prev, anchor: prev.cursorIndex }
    })
  }

  const clearSelection = () => {
    setState((prev) => {
      if (!prev) return prev
      return { ...prev, anchor: null }
    })
  }

  const selectionRange: SelectionRange | null = useMemo(() => {
    if (!state || state.anchor == null) return null
    const clampedCursor = Math.min(
      state.cursorIndex,
      Math.max(0, rowLayout.count - 1),
    )
    return {
      start: Math.min(state.anchor, clampedCursor),
      end: Math.max(state.anchor, clampedCursor),
    }
  }, [state, rowLayout.count])

  const highlightProps: SelectionHighlightProps | undefined = useMemo(() => {
    if (!state) return undefined
    const clampedCursor = Math.min(
      state.cursorIndex,
      Math.max(0, rowLayout.count - 1),
    )
    return {
      elementRowOffsets: rowLayout.offsets,
      cursorIndex: clampedCursor,
      selectionRange,
    }
  }, [state, rowLayout, selectionRange])

  const toCommentLineState = (): CommentLineState => {
    if (!state) return null
    return selectionToCommentLineState(
      state.cursorIndex,
      state.anchor,
      elements,
      diffViewMode,
    )
  }

  const resolveLineToGlobalIndex = (
    line: number,
    side: "LEFT" | "RIGHT",
  ): number | null => lineToGlobalIndex(line, side, elements, diffViewMode)

  return {
    state,
    rowLayout,
    selectionRange,
    moveCursor,
    moveCursorBy,
    startSelect,
    selectTo,
    toggleSelect,
    clearSelection,
    highlightProps,
    toCommentLineState,
    resolveLineToGlobalIndex,
    resolveGlobalIndex: (index: number) =>
      resolveGlobalIndex(index, elements, diffViewMode),
    resolveGlobalRangeToRegion: (startIdx: number, endIdx: number) =>
      resolveGlobalRangeToRegion(startIdx, endIdx, elements, diffViewMode),
  }
}

export type UseLineSelectionReturn = ReturnType<typeof useLineSelection>
