import { useCallback, useEffect, useMemo } from "react"
import { useHotkeys } from "react-hotkeys-hook"

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

export function useLineMode({
  elements,
  diffViewMode,
  containerRef,
  state,
  setState,
  onExit,
}: {
  elements: DiffElement[]
  diffViewMode: DiffViewMode
  containerRef: React.RefObject<HTMLElement | null>
  state: LineModeState | null
  setState: React.Dispatch<React.SetStateAction<LineModeState | null>>
  onExit: () => void
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
  useHotkeys("space", (e) => e.preventDefault(), {
    enabled: state !== null,
  })

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
