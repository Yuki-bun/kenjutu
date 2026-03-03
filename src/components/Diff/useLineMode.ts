import { useHotkey, useHotkeySequence } from "@tanstack/react-hotkeys"
import { useRef } from "react"

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
  // eslint-disable-next-line react-hooks/refs -- sync ref for stable hotkey closures, not used during render
  selectionRef.current = selection

  const hotkeyGuard = {
    enabled: active,
    target: containerRef,
  }

  useHotkey(
    "J",
    () => {
      selectionRef.current.moveCursorBy(1)
    },
    hotkeyGuard,
  )
  useHotkey(
    "K",
    () => {
      selectionRef.current.moveCursorBy(-1)
    },
    hotkeyGuard,
  )

  useHotkeySequence(
    ["G", "G"],
    () => selectionRef.current.moveToTop(),
    hotkeyGuard,
  )
  useHotkey("Shift+G", () => selectionRef.current.moveToBottom(), hotkeyGuard)

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

  useHotkey("N", () => selectionRef.current.moveToNextHunk(), hotkeyGuard)
  useHotkey("Shift+N", () => selectionRef.current.moveToPrevHunk(), hotkeyGuard)

  useHotkey("V", () => selectionRef.current.toggleSelect(), hotkeyGuard)

  useHotkey(
    "Space",
    () => {
      if (onMarkRegion) {
        const region = selectionRef.current.hunkId()
        if (region) {
          onMarkRegion(region)
          selectionRef.current.clearSelection()
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
