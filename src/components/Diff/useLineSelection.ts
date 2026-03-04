import { useEffect, useMemo } from "react"

import { DiffLine, DiffLineType, RegionId } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import { CommentLineState } from "./types"
import { DiffViewMode } from "./useDiffViewMode"

export type LineSelectionState = {
  cursor: CursorPosition
  /** null = no selection */
  anchor: CursorPosition | null
}

export type CursorPosition = {
  line: number
  side: "LEFT" | "RIGHT"
}

export type SelectionRange = {
  left: { start: number; end: number } | null
  right: { start: number; end: number } | null
}

export function getSelectedRegion(
  selection: LineSelectionState | null,
  elements: DiffElement[],
) {
  if (!selection) return { left: null, right: null }
  const flatElements = elements.flatMap((el) =>
    el.type === "hunk" ? el.hunk.lines : [],
  )
  const anchor = selection.anchor
  const anchorIdxRaw = anchor
    ? flatElements.findIndex((line) => isCursorLine(anchor, line))
    : null
  const anchorIdx =
    anchorIdxRaw != null && anchorIdxRaw !== -1 ? anchorIdxRaw : null
  const cursorIdx = flatElements.findIndex((line) =>
    isCursorLine(selection.cursor, line),
  )
  if (cursorIdx === -1) return { left: null, right: null }
  const range: SelectionRange = { left: null, right: null }

  const start = anchorIdx != null ? Math.min(anchorIdx, cursorIdx) : cursorIdx
  const end = anchorIdx != null ? Math.max(anchorIdx, cursorIdx) : cursorIdx
  for (let i = start; i <= end; i++) {
    const line = flatElements[i]
    if (line.lineType === "context") {
      range.left = {
        start: range.left?.start ?? line.oldLineno!,
        end: line.oldLineno!,
      }
      range.right = {
        start: range.right?.start ?? line.newLineno!,
        end: line.newLineno!,
      }
    } else if (line.lineType === "deletion") {
      range.left = {
        start: range.left?.start ?? line.oldLineno!,
        end: line.oldLineno!,
      }
    } else {
      range.right = {
        start: range.right?.start ?? line.newLineno!,
        end: line.newLineno!,
      }
    }
  }

  return range
}

function selectionToCommentLineState(
  selection: LineSelectionState,
  elements: DiffElement[],
): CommentLineState {
  const { anchor, cursor } = selection
  if (!anchor) {
    return {
      line: selection.cursor.line,
      side: selection.cursor.side,
    }
  }

  const flatElements = elements.flatMap((el) =>
    el.type === "hunk" ? el.hunk.lines : [],
  )
  const anchorIdx = flatElements.findIndex((line) => {
    const pos = diffLineToCursorPosition(line)
    return pos.line === anchor.line && pos.side === anchor.side
  })
  const cursorIdx = flatElements.findIndex((line) => {
    const pos = diffLineToCursorPosition(line)
    return (
      pos.line === selection.cursor.line && pos.side === selection.cursor.side
    )
  })
  const [start, end] =
    anchorIdx < cursorIdx ? [anchor, cursor] : [cursor, anchor]
  // TODO: handle github style old-new mixed selection and kenjutu style single
  // side selection
  return {
    line: end.line,
    side: end.side,
    startLine: start.line,
    startSide: start.side,
  }
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

function isLeftLineType(lineType: DiffLineType): boolean {
  return (
    lineType === "context" || lineType === "deletion" || lineType === "deleofnl"
  )
}

function isRightLineType(lineType: DiffLineType): boolean {
  return (
    lineType === "context" || lineType === "addition" || lineType === "addeofnl"
  )
}

function isCursorLine(cursor: CursorPosition, line: DiffLine): boolean {
  switch (cursor.side) {
    case "LEFT":
      return isLeftLineType(line.lineType) && line.oldLineno === cursor.line
    case "RIGHT":
      return isRightLineType(line.lineType) && line.newLineno === cursor.line
  }
}

export function computeRegionId(
  selectionRange: SelectionRange,
  elements: DiffElement[],
): RegionId | null {
  const left = selectionRange.left
  const right = selectionRange.right
  if (left && right) {
    return {
      oldStart: left.start,
      oldLines: left.end - left.start + 1,
      newStart: right.start,
      newLines: right.end - right.start + 1,
    }
  }
  if (left) {
    return {
      oldStart: left.start,
      oldLines: left.end - left.start + 1,
      newStart: 0,
      newLines: 0,
    }
  }

  if (right) {
    const flatElements = elements.flatMap((el) =>
      el.type === "hunk" ? el.hunk.lines : [],
    )
    const lineIdx = flatElements.findIndex(
      (line) =>
        line.newLineno === right.start && isRightLineType(line.lineType),
    )
    let oldStart = 0
    for (let i = lineIdx; i >= 0; i--) {
      const line = flatElements[i]
      if (isLeftLineType(line.lineType) && line.oldLineno != null) {
        oldStart = line.oldLineno
        break
      }
    }
    return {
      oldStart,
      oldLines: 0,
      newStart: right.start,
      newLines: right.end - right.start + 1,
    }
  }

  return null
}

export function diffLineToCursorPosition(line: DiffLine): CursorPosition {
  if (isLeftLineType(line.lineType)) {
    return { line: line.oldLineno!, side: "LEFT" }
  }
  return { line: line.newLineno!, side: "RIGHT" }
}

export function useLineSelection({
  elements,
  state,
  setState,
  containerRef,
}: {
  elements: DiffElement[]
  diffViewMode: DiffViewMode
  state: LineSelectionState | null
  setState: React.Dispatch<React.SetStateAction<LineSelectionState | null>>
  containerRef?: React.RefObject<HTMLElement | null>
}) {
  const isCursorValid = useMemo(() => {
    if (!state) return true
    const flatElements = elements.flatMap((el) =>
      el.type === "hunk" ? el.hunk.lines : [],
    )
    return flatElements.some((line) => isCursorLine(state.cursor, line))
  }, [elements, state])

  const isAnchorValid = useMemo(() => {
    if (!state || !state.anchor) return true
    const flatElements = elements.flatMap((el) =>
      el.type === "hunk" ? el.hunk.lines : [],
    )
    return flatElements.some((line) => isCursorLine(state.anchor!, line))
  }, [elements, state])

  const setCursor = (line: DiffLine) => {
    setState({ anchor: null, cursor: diffLineToCursorPosition(line) })
  }

  const moveCursor = (line: DiffLine) => {
    setState((prev) => {
      if (!prev) return prev
      return { ...prev, cursor: diffLineToCursorPosition(line) }
    })
  }

  const moveCursorBy = (delta: number) => {
    setState((prev) => {
      if (!prev) return prev
      const flattened = elements.flatMap((el) =>
        el.type === "hunk" ? el.hunk.lines : [],
      )
      const currentIndex = flattened.findIndex((line) =>
        isCursorLine(prev.cursor, line),
      )
      if (currentIndex === -1) return prev
      const nextIndex = Math.max(
        0,
        Math.min(currentIndex + delta, flattened.length - 1),
      )
      const nextLine = flattened[nextIndex]
      return { ...prev, cursor: diffLineToCursorPosition(nextLine) }
    })
  }

  const moveToBottom = () => {
    setState((prev) => {
      if (!prev) return prev
      const flattened = elements.flatMap((el) =>
        el.type === "hunk" ? el.hunk.lines : [],
      )
      const lastLine = flattened[flattened.length - 1]
      return { ...prev, cursor: diffLineToCursorPosition(lastLine) }
    })
  }

  const moveToTop = () => {
    setState((prev) => {
      if (!prev) return prev
      const flattened = elements.flatMap((el) =>
        el.type === "hunk" ? el.hunk.lines : [],
      )
      const firstLine = flattened[0]
      return { ...prev, cursor: diffLineToCursorPosition(firstLine) }
    })
  }

  const moveToNextHunk = () => {
    setState((prev) => {
      if (!prev) return prev
      const hunks = elements.filter((el) => el.type === "hunk")
      if (hunks.length === 0) return prev
      const hunkIdx = hunks.findIndex((h) =>
        h.hunk.lines.some((line) => isCursorLine(prev.cursor, line)),
      )
      const nextHunkIdx = Math.min(hunkIdx + 1, hunks.length - 1)
      const nextLine = hunks[nextHunkIdx].hunk.lines[0]
      return { ...prev, cursor: diffLineToCursorPosition(nextLine) }
    })
  }

  const moveToPrevHunk = () => {
    setState((prev) => {
      if (!prev) return prev
      const hunks = elements.filter((el) => el.type === "hunk")
      if (hunks.length === 0) return prev
      const hunkIdx = hunks.findIndex((h) =>
        h.hunk.lines.some((line) => isCursorLine(prev.cursor, line)),
      )
      const prevHunkIdx = Math.max(hunkIdx - 1, 0)
      const prevLine = hunks[prevHunkIdx].hunk.lines[0]
      return { ...prev, cursor: diffLineToCursorPosition(prevLine) }
    })
  }

  const startSelect = (line: DiffLine) => {
    setState((prev) => {
      if (!prev) return prev
      const pos = diffLineToCursorPosition(line)
      return { anchor: pos, cursor: pos }
    })
  }

  const toggleSelect = () => {
    setState((prev) => {
      if (!prev) return prev
      if (prev.anchor != null) {
        return { ...prev, anchor: null }
      }
      return { ...prev, anchor: prev.cursor }
    })
  }

  const clearSelection = () => {
    setState((prev) => {
      if (!prev) return prev
      return { ...prev, anchor: null }
    })
  }

  useEffect(() => {
    if (!isCursorValid) {
      moveToTop()
    }
  }, [isCursorValid, moveToTop])

  useEffect(() => {
    if (!isAnchorValid) {
      clearSelection()
    }
  }, [isAnchorValid, clearSelection])

  const cursor = state?.cursor
  useEffect(() => {
    if (!cursor) return
    const container = containerRef?.current
    if (!container) return
    requestAnimationFrame(() => {
      const el = container.querySelector("[data-cursor]")
      el?.scrollIntoView({ behavior: "instant", block: "nearest" })
    })
  }, [cursor, containerRef])

  const selectionRange = getSelectedRegion(state, elements)

  const toCommentLineState = (): CommentLineState => {
    if (!state) return null
    return selectionToCommentLineState(state, elements)
  }

  const regionId = () => computeRegionId(selectionRange, elements)

  return {
    state,
    selectionRange,
    setCursor,
    moveCursor,
    moveCursorBy,
    moveToBottom,
    moveToTop,
    moveToNextHunk,
    moveToPrevHunk,
    startSelect,
    toggleSelect,
    clearSelection,
    toCommentLineState,
    regionId,
  }
}

export type UseLineSelectionReturn = ReturnType<typeof useLineSelection>
