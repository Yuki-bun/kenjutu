import { useEffect, useMemo, useRef } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { DiffLine, HunkId } from "@/bindings"

import { DiffElement } from "./hunkGaps"
import { PairedLine, pairLinesForSplitView } from "./SplitDiff"
import { CommentLineState } from "./types"
import { DiffViewMode } from "./useDiffViewMode"

export type LineIdentity = {
  line: number
  side: "LEFT" | "RIGHT"
}

export type SelectionState =
  | { isSelecting: false }
  | { isSelecting: true; anchor: LineIdentity }

export type LineModeState = {
  cursor: LineIdentity
  selection: SelectionState
}

export type LineCursorProps = {
  readonly cursorKey: string
  readonly selectedKeys: ReadonlySet<string>
}

export type LineModeControl = {
  state: LineModeState | null
  setState: React.Dispatch<React.SetStateAction<LineModeState | null>>
  onExit: () => void
}

export function lineKey(id: LineIdentity): string {
  return `${id.side}:${id.line}`
}

export function lineIdentityForDiffLine(line: DiffLine): LineIdentity | null {
  if (line.lineType === "deletion") {
    return line.oldLineno != null
      ? { line: line.oldLineno, side: "LEFT" }
      : null
  }
  const lineNumber = line.newLineno ?? line.oldLineno
  return lineNumber != null ? { line: lineNumber, side: "RIGHT" } : null
}

export function lineIdentityForPairedLine(
  pair: PairedLine,
): LineIdentity | null {
  if (pair.right?.newLineno != null) {
    return { line: pair.right.newLineno, side: "RIGHT" }
  }
  if (pair.left?.oldLineno != null) {
    return { line: pair.left.oldLineno, side: "LEFT" }
  }
  return null
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

// --- Navigation helpers (derive from elements on demand) ---

function collectIdentities(
  elements: DiffElement[],
  mode: DiffViewMode,
): LineIdentity[] {
  const result: LineIdentity[] = []
  for (const el of elements) {
    if (el.type !== "hunk") continue
    if (mode === "split") {
      for (const pair of pairLinesForSplitView(el.hunk.lines)) {
        const id = lineIdentityForPairedLine(pair)
        if (id) result.push(id)
      }
    } else {
      for (const line of el.hunk.lines) {
        const id = lineIdentityForDiffLine(line)
        if (id) result.push(id)
      }
    }
  }
  return result
}

function moveBy(
  cursor: LineIdentity,
  elements: DiffElement[],
  mode: DiffViewMode,
  count: number,
): LineIdentity | null {
  const ids = collectIdentities(elements, mode)
  if (ids.length === 0) return null
  const ck = lineKey(cursor)
  const idx = ids.findIndex((id) => lineKey(id) === ck)
  if (idx === -1) return ids[0]
  const target = Math.max(0, Math.min(ids.length - 1, idx + count))
  return ids[target]
}

function moveToBoundary(
  elements: DiffElement[],
  mode: DiffViewMode,
  which: "first" | "last",
): LineIdentity | null {
  const ids = collectIdentities(elements, mode)
  if (ids.length === 0) return null
  return which === "first" ? ids[0] : ids[ids.length - 1]
}

function moveToAdjacentHunk(
  cursor: LineIdentity,
  elements: DiffElement[],
  mode: DiffViewMode,
  direction: "next" | "prev",
): LineIdentity | null {
  const ck = lineKey(cursor)

  const hunks: {
    firstLine: LineIdentity
    cursorAtFirst: boolean
    containsCursor: boolean
  }[] = []

  for (const el of elements) {
    if (el.type !== "hunk") continue
    let firstLine: LineIdentity | null = null
    let containsCursor = false
    let cursorAtFirst = false

    if (mode === "split") {
      for (const pair of pairLinesForSplitView(el.hunk.lines)) {
        const id = lineIdentityForPairedLine(pair)
        if (!id) continue
        if (!firstLine) firstLine = id
        if (lineKey(id) === ck) {
          containsCursor = true
          if (firstLine === id) cursorAtFirst = true
        }
      }
    } else {
      for (const line of el.hunk.lines) {
        const id = lineIdentityForDiffLine(line)
        if (!id) continue
        if (!firstLine) firstLine = id
        if (lineKey(id) === ck) {
          containsCursor = true
          if (firstLine === id) cursorAtFirst = true
        }
      }
    }

    if (firstLine) {
      hunks.push({ firstLine, cursorAtFirst, containsCursor })
    }
  }

  const cursorHunkIdx = hunks.findIndex((h) => h.containsCursor)
  if (cursorHunkIdx === -1) return null

  if (direction === "next") {
    return cursorHunkIdx + 1 < hunks.length
      ? hunks[cursorHunkIdx + 1].firstLine
      : null
  }

  // prev: if cursor is not at the hunk's first line, jump to start of current hunk
  if (!hunks[cursorHunkIdx].cursorAtFirst) {
    return hunks[cursorHunkIdx].firstLine
  }
  // cursor is at start of hunk — jump to previous hunk
  return cursorHunkIdx > 0 ? hunks[cursorHunkIdx - 1].firstLine : null
}

function clampCursor(
  cursor: LineIdentity,
  elements: DiffElement[],
  mode: DiffViewMode,
): LineIdentity | null {
  const ids = collectIdentities(elements, mode)
  if (ids.length === 0) return null
  const ck = lineKey(cursor)
  const found = ids.find((id) => lineKey(id) === ck)
  return found ?? ids[0]
}

// --- Selection region resolution ---

function computeSelectedKeys(
  anchor: LineIdentity,
  cursor: LineIdentity,
  elements: DiffElement[],
  mode: DiffViewMode,
): Set<string> {
  const ak = lineKey(anchor)
  const ck = lineKey(cursor)
  if (ak === ck) return new Set([ak])

  const keys = new Set<string>()
  let inRange = false
  let entryKey: string | null = null

  for (const el of elements) {
    if (el.type !== "hunk") continue
    if (mode === "split") {
      for (const pair of pairLinesForSplitView(el.hunk.lines)) {
        const id = lineIdentityForPairedLine(pair)
        if (!id) continue
        const k = lineKey(id)
        if (!inRange && (k === ak || k === ck)) {
          inRange = true
          entryKey = k
        }
        if (inRange) {
          keys.add(k)
          if ((k === ak || k === ck) && k !== entryKey) return keys
        }
      }
    } else {
      for (const line of el.hunk.lines) {
        const id = lineIdentityForDiffLine(line)
        if (!id) continue
        const k = lineKey(id)
        if (!inRange && (k === ak || k === ck)) {
          inRange = true
          entryKey = k
        }
        if (inRange) {
          keys.add(k)
          if ((k === ak || k === ck) && k !== entryKey) return keys
        }
      }
    }
  }
  return keys
}

export function resolveSelectionToRegion(
  id1: LineIdentity,
  id2: LineIdentity,
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
): HunkId | null {
  const k1 = lineKey(id1)
  const k2 = lineKey(id2)
  const isSingle = k1 === k2

  let minOld = Infinity
  let maxOld = -Infinity
  let minNew = Infinity
  let maxNew = -Infinity
  let lastOldBefore: number | null = null
  let lastNewBefore: number | null = null
  let hasChange = false

  // State machine: "before" → "in" → "done"
  let phase: "before" | "in" | "done" = "before"
  let entryKey: string | null = null

  for (const el of elements) {
    if (el.type !== "hunk" || phase === "done") continue

    if (diffViewMode === "split") {
      for (const pair of pairLinesForSplitView(el.hunk.lines)) {
        if (phase === "done") break
        const id = lineIdentityForPairedLine(pair)
        if (!id) continue
        const k = lineKey(id)
        const isEndpoint = k === k1 || k === k2

        if (phase === "before") {
          if (isEndpoint) {
            phase = "in"
            entryKey = k
          } else {
            if (pair.left?.oldLineno != null)
              lastOldBefore = pair.left.oldLineno
            if (pair.right?.newLineno != null)
              lastNewBefore = pair.right.newLineno
            continue
          }
        }

        // In range
        if (pair.left) {
          if (
            pair.left.lineType === "addition" ||
            pair.left.lineType === "deletion"
          )
            hasChange = true
          if (pair.left.oldLineno != null) {
            minOld = Math.min(minOld, pair.left.oldLineno)
            maxOld = Math.max(maxOld, pair.left.oldLineno)
          }
        }
        if (pair.right) {
          if (
            pair.right.lineType === "addition" ||
            pair.right.lineType === "deletion"
          )
            hasChange = true
          if (pair.right.newLineno != null) {
            minNew = Math.min(minNew, pair.right.newLineno)
            maxNew = Math.max(maxNew, pair.right.newLineno)
          }
        }

        if (isEndpoint && (isSingle || k !== entryKey)) {
          phase = "done"
        }
      }
    } else {
      for (const line of el.hunk.lines) {
        if (phase === "done") break
        const id = lineIdentityForDiffLine(line)
        if (!id) continue
        const k = lineKey(id)
        const isEndpoint = k === k1 || k === k2

        const isOldSide =
          line.lineType === "context" || line.lineType === "deletion"
        const isNewSide =
          line.lineType === "context" || line.lineType === "addition"

        if (phase === "before") {
          if (isEndpoint) {
            phase = "in"
            entryKey = k
          } else {
            if (isOldSide && line.oldLineno != null)
              lastOldBefore = line.oldLineno
            if (isNewSide && line.newLineno != null)
              lastNewBefore = line.newLineno
            continue
          }
        }

        // In range
        if (line.lineType === "addition" || line.lineType === "deletion")
          hasChange = true
        if (isOldSide && line.oldLineno != null) {
          minOld = Math.min(minOld, line.oldLineno)
          maxOld = Math.max(maxOld, line.oldLineno)
        }
        if (isNewSide && line.newLineno != null) {
          minNew = Math.min(minNew, line.newLineno)
          maxNew = Math.max(maxNew, line.newLineno)
        }

        if (isEndpoint && (isSingle || k !== entryKey)) {
          phase = "done"
        }
      }
    }
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

// --- Hook ---

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
  const stateRef = useRef(state)
  stateRef.current = state

  const moveCursor = (delta: number) => {
    setState((prev) => {
      if (!prev) return prev
      const next = moveBy(prev.cursor, elements, diffViewMode, delta)
      if (!next || lineKey(next) === lineKey(prev.cursor)) return prev
      return { ...prev, cursor: next }
    })
  }

  const setCursor = (target: LineIdentity) => {
    setState((prev) => {
      if (!prev) return prev
      if (lineKey(target) === lineKey(prev.cursor)) return prev
      return { ...prev, cursor: target }
    })
  }

  const cursor = state?.cursor
  useEffect(() => {
    if (!cursor) return
    const container = containerRef.current
    if (!container) return
    const ck = lineKey(cursor)
    requestAnimationFrame(() => {
      const el = container.querySelector(`[data-line-id="${ck}"]`)
      el?.scrollIntoView({ behavior: "instant", block: "nearest" })
    })
  }, [cursor, containerRef])

  useHotkeys("j", () => moveCursor(1), { enabled: state !== null })
  useHotkeys("k", () => moveCursor(-1), { enabled: state !== null })

  useHotkeys(
    "shift+g",
    () => {
      const target = moveToBoundary(elements, diffViewMode, "last")
      if (target) setCursor(target)
    },
    { enabled: state !== null },
  )

  const HALF_PAGE = 20
  useHotkeys(
    "g>g",
    () => {
      const target = moveToBoundary(elements, diffViewMode, "first")
      if (target) setCursor(target)
    },
    { enabled: state !== null },
  )

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

  useHotkeys(
    "n",
    () => {
      const st = stateRef.current
      if (!st) return
      const target = moveToAdjacentHunk(
        st.cursor,
        elements,
        diffViewMode,
        "next",
      )
      if (target) setCursor(target)
    },
    { enabled: state !== null },
  )
  useHotkeys(
    "shift+n",
    () => {
      const st = stateRef.current
      if (!st) return
      const target = moveToAdjacentHunk(
        st.cursor,
        elements,
        diffViewMode,
        "prev",
      )
      if (target) setCursor(target)
    },
    { enabled: state !== null },
  )

  useHotkeys(
    "space",
    (e) => {
      e.preventDefault()
      const st = stateRef.current
      if (!st || !onMarkRegion) return

      const anchor = st.selection.isSelecting ? st.selection.anchor : st.cursor
      const region = resolveSelectionToRegion(
        anchor,
        st.cursor,
        elements,
        diffViewMode,
      )
      if (region) {
        onMarkRegion(region)
        if (st.selection.isSelecting) {
          // Determine which identity comes first to reset cursor to the top
          const ids = collectIdentities(elements, diffViewMode)
          const anchorIdx = ids.findIndex(
            (id) => lineKey(id) === lineKey(anchor),
          )
          const cursorIdx = ids.findIndex(
            (id) => lineKey(id) === lineKey(st.cursor),
          )
          const topCursor = anchorIdx <= cursorIdx ? anchor : st.cursor
          setState((prev) =>
            prev
              ? { cursor: topCursor, selection: { isSelecting: false } }
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
          selection: { isSelecting: true, anchor: prev.cursor },
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

      const { cursor: cur } = st

      if (st.selection.isSelecting) {
        const { anchor } = st.selection
        if (anchor.side === cur.side) {
          const startLine = Math.min(anchor.line, cur.line)
          const endLine = Math.max(anchor.line, cur.line)
          if (startLine !== endLine) {
            onComment({
              line: endLine,
              side: cur.side,
              startLine,
              startSide: cur.side,
            })
            return
          }
        }
      }

      onComment({ line: cur.line, side: cur.side })
    },
    { enabled: state !== null && onComment != null },
  )

  useHotkeys("escape", () => onExit(), { enabled: state !== null })

  const lineCursor: LineCursorProps | undefined = useMemo(() => {
    if (!state) return undefined
    const clamped = clampCursor(state.cursor, elements, diffViewMode)
    if (!clamped) return undefined

    const cursorK = lineKey(clamped)
    const selectedKeys: Set<string> = state.selection.isSelecting
      ? computeSelectedKeys(
          state.selection.anchor,
          clamped,
          elements,
          diffViewMode,
        )
      : new Set()

    return { cursorKey: cursorK, selectedKeys }
  }, [state, elements, diffViewMode])

  return { lineCursor }
}
