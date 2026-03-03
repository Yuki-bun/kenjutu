import { useEffect, useRef, useState } from "react"

import { DiffLine } from "@/bindings"

import { UseLineSelectionReturn } from "./useLineSelection"

export function useLineDrag({
  selection,
  enabled,
  onActivate,
}: {
  selection: UseLineSelectionReturn
  enabled: boolean
  /** Called when the user clicks a row but line mode is not yet active. */
  onActivate?: (line: DiffLine) => void
}) {
  const [isDragging, setIsDragging] = useState(false)
  const didDragRef = useRef(false)

  const onRowMouseDown = enabled
    ? (line: DiffLine) => {
        if (selection.state == null) {
          onActivate?.(line)
          return
        }
        didDragRef.current = false
        setIsDragging(true)
        selection.startSelect(line)
      }
    : undefined

  const onRowMouseEnter = enabled
    ? (line: DiffLine) => {
        if (!isDragging) return
        didDragRef.current = true
        selection.moveCursor(line)
      }
    : undefined

  const onRowMouseUp = enabled
    ? () => {
        if (!isDragging) return
        setIsDragging(false)
        // If user clicked without dragging, collapse to a plain cursor
        if (!didDragRef.current) {
          selection.clearSelection()
        }
      }
    : undefined

  // End drag on mouseup anywhere (safety net for releasing outside rows)
  useEffect(() => {
    const handleMouseUp = () => {
      if (isDragging) {
        setIsDragging(false)
        if (!didDragRef.current) {
          selection.clearSelection()
        }
      }
    }
    document.addEventListener("mouseup", handleMouseUp)
    return () => document.removeEventListener("mouseup", handleMouseUp)
  }, [isDragging, selection])

  return {
    isDragging,
    onRowMouseDown,
    onRowMouseEnter,
    onRowMouseUp,
  }
}
