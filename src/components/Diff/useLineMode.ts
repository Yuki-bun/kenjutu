import { useCallback, useEffect, useMemo, useRef } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { HunkId } from "@/bindings"

import { CommentLineState } from "./FileDiffItem"
import { DiffElement } from "./hunkGaps"
import { pairLinesForSplitView } from "./SplitDiff"
import { DiffViewMode } from "./useDiffViewMode"

export type SelectionState =
  | { isSelecting: false }
  | { isSelecting: true; anchorIndex: number }

export type LineModeState = {
  cursorIndex: number
  selection: SelectionState
}

export type LineCursorProps = {
  readonly elementRowOffsets: ReadonlyMap<number, number>
  readonly cursorIndex: number
  readonly selectedIndices: ReadonlySet<number>
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
    const rowCount =
      diffViewMode === "split"
        ? pairLinesForSplitView(el.hunk.lines).length
        : el.hunk.lines.length
    if (globalIndex < offset + rowCount) {
      const localIndex = globalIndex - offset
      if (diffViewMode === "split") {
        const pair = pairLinesForSplitView(el.hunk.lines)[localIndex]
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

function resolveGlobalRangeToRegion(
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

  let offset = 0
  for (const el of elements) {
    if (el.type !== "hunk") continue
    const lines = el.hunk.lines
    const rowCount =
      diffViewMode === "split"
        ? pairLinesForSplitView(lines).length
        : lines.length

    for (let localIdx = 0; localIdx < rowCount; localIdx++) {
      const gi = offset + localIdx

      if (diffViewMode === "split") {
        const pair = pairLinesForSplitView(lines)[localIdx]
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
        }
      } else {
        const line = lines[localIdx]
        if (gi < startIndex) {
          if (line.oldLineno != null) lastOldBefore = line.oldLineno
          if (line.newLineno != null) lastNewBefore = line.newLineno
        } else if (gi <= endIndex) {
          if (line.oldLineno != null) {
            minOld = Math.min(minOld, line.oldLineno)
            maxOld = Math.max(maxOld, line.oldLineno)
          }
          if (line.newLineno != null) {
            minNew = Math.min(minNew, line.newLineno)
            maxNew = Math.max(maxNew, line.newLineno)
          }
        }
      }
    }

    offset += rowCount
    if (offset > endIndex) break
  }

  const hasOld = minOld !== Infinity
  const hasNew = minNew !== Infinity
  if (!hasOld && !hasNew) return null

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
    for (let i = 0; i < elements.length; i++) {
      const el = elements[i]
      if (el.type !== "hunk") continue
      const rowCount =
        diffViewMode === "split"
          ? pairLinesForSplitView(el.hunk.lines).length
          : el.hunk.lines.length
      offsets.set(i, count)
      count += rowCount
    }
    return { count, offsets }
  }, [elements, diffViewMode])

  const stateRef = useRef(state)
  stateRef.current = state

  const moveCursor = useCallback(
    (delta: number) => {
      setState((prev) => {
        if (!prev) return prev
        const next = Math.max(
          0,
          Math.min(prev.cursorIndex + delta, totalRows.count - 1),
        )
        if (next === prev.cursorIndex) return prev
        return { ...prev, cursorIndex: next }
      })
    },
    [totalRows.count, setState],
  )

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
            prev ? { ...prev, selection: { isSelecting: false } } : prev,
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
    const selectedIndices = new Set<number>()
    if (state.selection.isSelecting) {
      const start = Math.min(state.selection.anchorIndex, clampedCursor)
      const end = Math.max(state.selection.anchorIndex, clampedCursor)
      for (let i = start; i <= end; i++) {
        selectedIndices.add(i)
      }
    }
    return {
      elementRowOffsets: totalRows.offsets,
      cursorIndex: clampedCursor,
      selectedIndices,
    }
  }, [state, totalRows])

  return { lineCursor }
}
