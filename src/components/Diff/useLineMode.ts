import { useHotkey, useHotkeySequence } from "@tanstack/react-hotkeys"
import { useEffect, useRef } from "react"

import { HunkId } from "@/bindings"

import { UseLineSelectionReturn } from "./useLineSelection"

export function useLineMode({
  selection,
  containerRef,
  active,
  onExit,
  onComment,
  onMarkRegion,
}: {
  selection: UseLineSelectionReturn
  containerRef: React.RefObject<HTMLElement | null>
  active: boolean
  onExit: () => void
  onComment?: () => void
  onMarkRegion?: (region: HunkId) => void
}) {
  // Keep a ref to selection for use in hotkey closures
  const selectionRef = useRef(selection)
  selectionRef.current = selection

  // Auto-scroll on cursor change
  const cursorIndex = selection.state?.cursorIndex
  useEffect(() => {
    if (cursorIndex == null) return
    const container = containerRef.current
    if (!container) return
    requestAnimationFrame(() => {
      const el = container.querySelector(`[data-nav-index="${cursorIndex}"]`)
      el?.scrollIntoView({ behavior: "instant", block: "nearest" })
    })
  }, [cursorIndex, containerRef])

  const hotkeyGuard = {
    enabled: active,
    target: containerRef,
  }

  useHotkey(
    "J",
    () => {
      const s = selectionRef.current
      if (s.state?.anchor != null) {
        // In selection mode, extend selection
        s.selectTo(Math.min(s.state.cursorIndex + 1, s.rowLayout.count - 1))
      } else {
        s.moveCursorBy(1)
      }
    },
    hotkeyGuard,
  )
  useHotkey(
    "K",
    () => {
      const s = selectionRef.current
      if (s.state?.anchor != null) {
        s.selectTo(Math.max(s.state.cursorIndex - 1, 0))
      } else {
        s.moveCursorBy(-1)
      }
    },
    hotkeyGuard,
  )

  useHotkeySequence(
    ["G", "G"],
    () => selectionRef.current.moveCursor(0),
    hotkeyGuard,
  )
  useHotkey(
    "Shift+G",
    () => {
      const s = selectionRef.current
      s.moveCursor(s.rowLayout.count - 1)
    },
    hotkeyGuard,
  )

  const HALF_PAGE = 20
  useHotkey(
    "Control+D",
    () => selectionRef.current.moveCursorBy(HALF_PAGE),
    hotkeyGuard,
  )
  useHotkey(
    "Control+U",
    () => selectionRef.current.moveCursorBy(-HALF_PAGE),
    hotkeyGuard,
  )

  useHotkey(
    "N",
    () => {
      const s = selectionRef.current
      if (!s.state) return
      const nextStart = s.rowLayout.hunkStarts.find(
        (hs) => hs > s.state!.cursorIndex,
      )
      if (nextStart != null) s.moveCursor(nextStart)
    },
    hotkeyGuard,
  )
  useHotkey(
    "Shift+N",
    () => {
      const s = selectionRef.current
      if (!s.state) return
      let prevStart: number | undefined
      for (const hs of s.rowLayout.hunkStarts) {
        if (hs >= s.state.cursorIndex) break
        prevStart = hs
      }
      if (prevStart != null) s.moveCursor(prevStart)
    },
    hotkeyGuard,
  )

  useHotkey("V", () => selectionRef.current.toggleSelect(), hotkeyGuard)

  useHotkey(
    "Space",
    () => {
      const s = selectionRef.current
      if (!s.state || !onMarkRegion) return
      const startIdx =
        s.state.anchor != null
          ? Math.min(s.state.anchor, s.state.cursorIndex)
          : s.state.cursorIndex
      const endIdx =
        s.state.anchor != null
          ? Math.max(s.state.anchor, s.state.cursorIndex)
          : s.state.cursorIndex
      const region = s.resolveGlobalRangeToRegion(startIdx, endIdx)
      if (region) {
        onMarkRegion(region)
        if (s.state.anchor != null) {
          // Reset cursor to top of selected region (region will be removed
          // from this side after marking)
          s.moveCursor(startIdx)
        }
      }
    },
    hotkeyGuard,
  )

  useHotkey(
    "C",
    () => {
      if (onComment) onComment()
    },
    { ...hotkeyGuard, enabled: active && onComment != null },
  )

  useHotkey("Escape", () => onExit(), hotkeyGuard)
}
