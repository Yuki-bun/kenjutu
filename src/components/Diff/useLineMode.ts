import { useEffect, useMemo, useRef } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { HunkId } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import { pairLinesForSplitView } from "./SplitDiff"
import { CommentLineState } from "./types"
import { DiffViewMode } from "./useDiffViewMode"

export type SelectionState =
  | { isSelecting: false }
  | { isSelecting: true; anchorIndex: number }

export type LineModeState = {
  cursorIndex: number
  selection: SelectionState
}

export type SelectionRange = {
  readonly start: number
  readonly end: number
}

export type LineCursorProps = {
  readonly elementRowOffsets: ReadonlyMap<number, number>
  readonly cursorIndex: number
  readonly selectionRange: SelectionRange | null
}

export type LineNavProps = {
  navIndex: number
  isCursor: boolean
  isSelected: boolean
}

export type LineModeControl = {
  state: LineModeState | null
  setState: React.Dispatch<React.SetStateAction<LineModeState | null>>
  onExit: () => void
}

export function getLineHighlightBg({
  isCursor,
  isSelected,
  isInRange,
  defaultBg,
}: {
  isCursor?: boolean
  isSelected?: boolean
  isInRange?: boolean
  defaultBg: string
}): string {
  if (isCursor) return "bg-yellow-200/80 dark:bg-yellow-700/40"
  if (isSelected) return "bg-yellow-100/60 dark:bg-yellow-800/30"
  if (isInRange) return "bg-blue-50 dark:bg-blue-950/30"
  return defaultBg
}

type ResolvedLine = {
  line: number
  side: "LEFT" | "RIGHT"
}

function resolveGlobalIndex(
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

export function useLineMode({
  elements,
  diffViewMode,
  containerRef,
  state,
  setState,
  onExit,
  onComment,
  onMarkRegion,
}: {
  elements: DiffElement[]
  diffViewMode: DiffViewMode
  containerRef: React.RefObject<HTMLElement | null>
  state: LineModeState | null
  setState: React.Dispatch<React.SetStateAction<LineModeState | null>>
  onExit: () => void
  onComment?: (comment: NonNullable<CommentLineState>) => void
  onMarkRegion?: (region: HunkId) => void
}) {
  const totalRows = useMemo(() => {
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
  }, [elements, diffViewMode])

  const stateRef = useRef(state)
  stateRef.current = state

  const moveCursor = (delta: number) => {
    setState((prev) => {
      if (!prev) return prev
      const next = Math.max(
        0,
        Math.min(prev.cursorIndex + delta, totalRows.count - 1),
      )
      if (next === prev.cursorIndex) return prev
      return { ...prev, cursorIndex: next }
    })
  }

  const setCursor = (index: number) => {
    setState((prev) => {
      if (!prev) return prev
      const clamped = Math.max(0, Math.min(index, totalRows.count - 1))
      if (clamped === prev.cursorIndex) return prev
      return { ...prev, cursorIndex: clamped }
    })
  }

  const cursorIndex = state?.cursorIndex
  useEffect(() => {
    if (cursorIndex == null) return
    const container = containerRef.current
    if (!container) return
    requestAnimationFrame(() => {
      const el = container.querySelector(`[data-nav-index="${cursorIndex}"]`)
      el?.scrollIntoView({ behavior: "instant", block: "nearest" })
    })
  }, [cursorIndex, containerRef])

  useHotkeys("j", () => moveCursor(1), { enabled: state !== null })
  useHotkeys("k", () => moveCursor(-1), { enabled: state !== null })

  // shift+g → jump to last line
  useHotkeys("shift+g", () => setCursor(totalRows.count - 1), {
    enabled: state !== null,
  })

  // g g → jump to first line
  const HALF_PAGE = 20
  useHotkeys("g>g", () => setCursor(0), { enabled: state !== null })

  // ctrl+d / ctrl+u → half-page jumps
  useHotkeys(
    "ctrl+d",
    (e) => {
      e.preventDefault()
      moveCursor(HALF_PAGE)
    },
    { enabled: state !== null },
  )
  useHotkeys(
    "ctrl+u",
    (e) => {
      e.preventDefault()
      moveCursor(-HALF_PAGE)
    },
    { enabled: state !== null },
  )

  // n → jump to next hunk start, shift+n → jump to previous hunk start
  useHotkeys(
    "n",
    () => {
      const st = stateRef.current
      if (!st) return
      const { hunkStarts } = totalRows
      const nextStart = hunkStarts.find((s) => s > st.cursorIndex)
      if (nextStart != null) setCursor(nextStart)
    },
    { enabled: state !== null },
  )
  useHotkeys(
    "shift+n",
    () => {
      const st = stateRef.current
      if (!st) return
      const { hunkStarts } = totalRows
      // Find the last hunk start before the current cursor
      let prevStart: number | undefined
      for (const s of hunkStarts) {
        if (s >= st.cursorIndex) break
        prevStart = s
      }
      if (prevStart != null) setCursor(prevStart)
    },
    { enabled: state !== null },
  )

  useHotkeys(
    "space",
    (e) => {
      e.preventDefault()
      const st = stateRef.current
      if (!st || !onMarkRegion) return
      let startIdx = st.cursorIndex
      let endIdx = st.cursorIndex
      if (st.selection.isSelecting) {
        startIdx = Math.min(st.selection.anchorIndex, st.cursorIndex)
        endIdx = Math.max(st.selection.anchorIndex, st.cursorIndex)
      }
      const region = resolveGlobalRangeToRegion(
        startIdx,
        endIdx,
        elements,
        diffViewMode,
      )
      if (region) {
        onMarkRegion(region)
        if (st.selection.isSelecting) {
          setState((prev) =>
            prev
              ? {
                  // rests cursor to the top of the selected region as the selected region
                  // will be removed from this side after marking
                  cursorIndex: prev.selection.isSelecting
                    ? Math.min(prev.selection.anchorIndex, prev.cursorIndex)
                    : prev.cursorIndex,
                  selection: { isSelecting: false },
                }
              : prev,
          )
        }
      }
    },
    { enabled: state !== null },
  )

  useHotkeys(
    "v",
    () => {
      setState((prev) => {
        if (!prev) return prev
        if (prev.selection.isSelecting) {
          return { ...prev, selection: { isSelecting: false } }
        }
        return {
          ...prev,
          selection: {
            isSelecting: true,
            anchorIndex: prev.cursorIndex,
          },
        }
      })
    },
    { enabled: state !== null },
  )

  useHotkeys(
    "c",
    (e) => {
      const st = stateRef.current
      if (!st || !onComment) return
      e.preventDefault()

      const cursorResolved = resolveGlobalIndex(
        st.cursorIndex,
        elements,
        diffViewMode,
      )
      if (!cursorResolved) return

      if (st.selection.isSelecting) {
        const anchorResolved = resolveGlobalIndex(
          st.selection.anchorIndex,
          elements,
          diffViewMode,
        )
        if (anchorResolved && anchorResolved.side === cursorResolved.side) {
          const startLine = Math.min(anchorResolved.line, cursorResolved.line)
          const endLine = Math.max(anchorResolved.line, cursorResolved.line)
          if (startLine !== endLine) {
            onComment({
              line: endLine,
              side: cursorResolved.side,
              startLine,
              startSide: cursorResolved.side,
            })
            return
          }
        }
      }

      onComment({ line: cursorResolved.line, side: cursorResolved.side })
    },
    { enabled: state !== null && onComment != null },
  )

  useHotkeys("escape", () => onExit(), { enabled: state !== null })

  const lineCursor: LineCursorProps | undefined = useMemo(() => {
    if (!state) return undefined
    const clampedCursor = Math.min(
      state.cursorIndex,
      Math.max(0, totalRows.count - 1),
    )
    const selectionRange: SelectionRange | null = state.selection.isSelecting
      ? {
          start: Math.min(state.selection.anchorIndex, clampedCursor),
          end: Math.max(state.selection.anchorIndex, clampedCursor),
        }
      : null
    return {
      elementRowOffsets: totalRows.offsets,
      cursorIndex: clampedCursor,
      selectionRange,
    }
  }, [state, totalRows])

  return { lineCursor }
}
