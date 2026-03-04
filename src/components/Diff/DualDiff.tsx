import { useHotkey } from "@tanstack/react-hotkeys"
import { useState } from "react"

import { DiffLine, RegionId } from "@/bindings"
import { cn } from "@/lib/utils"

import { DiffElement } from "./hunkGaps"
import { UnifiedHunkLines } from "./UnifiedDiff"
import { useLineDrag } from "./useLineDrag"
import { useLineMode } from "./useLineMode"
import {
  CursorPosition,
  diffLineToCursorPosition,
  LineSelectionControl,
  SelectionRange,
  useLineSelection,
} from "./useLineSelection"

export type DualDiffPanel = "remaining" | "reviewed"

type DualDiffProps = {
  remainingElements: DiffElement[]
  reviewedElements: DiffElement[]
  lineSelection?: LineSelectionControl
  onMarkRegion?: (region: RegionId, panel: DualDiffPanel) => void
  fileItemRef: React.RefObject<HTMLDivElement | null>
}

export function DualDiff({
  remainingElements,
  reviewedElements,
  lineSelection,
  onMarkRegion,
  fileItemRef,
}: DualDiffProps) {
  const [activePanel, setActivePanel] = useState<DualDiffPanel>("remaining")

  const isLineModeActive =
    lineSelection?.state !== null && lineSelection?.state !== undefined

  const activeElements =
    activePanel === "remaining" ? remainingElements : reviewedElements

  useHotkey(
    "Tab",
    () => {
      const next =
        activePanel === "remaining" ? "reviewed" : ("remaining" as const)
      setActivePanel(next)
      const nextElements =
        next === "remaining" ? remainingElements : reviewedElements
      const firstLine = nextElements.find((el) => el.type === "hunk")?.hunk
        .lines[0]
      const cursorPos = firstLine ? diffLineToCursorPosition(firstLine) : null
      if (cursorPos) {
        lineSelection?.setState({
          cursor: cursorPos,
          anchor: null,
        })
      }
    },
    { enabled: isLineModeActive },
  )

  const handleMarkRegionForPanel = onMarkRegion
    ? (region: RegionId) => onMarkRegion(region, activePanel)
    : undefined

  const selection = useLineSelection({
    elements: activeElements,
    diffViewMode: "unified",
    state: lineSelection?.state ?? null,
    setState: lineSelection?.setState ?? (() => {}),
    containerRef: fileItemRef,
  })

  const drag = useLineDrag({
    selection,
    enabled: true,
    onActivate: (line) => {
      selection.setCursor(line)
    },
  })

  const onRowMouseDown = (side: DualDiffPanel) => (line: DiffLine) => {
    if (activePanel !== side) {
      setActivePanel(side)
      selection.setCursor(line)
    } else {
      drag.onRowMouseDown?.(line)
    }
    fileItemRef.current?.focus()
  }

  useLineMode({
    selection,
    containerRef: fileItemRef,
    active: isLineModeActive,
    onExit: lineSelection?.onExit ?? (() => {}),
    onMarkRegion: handleMarkRegionForPanel,
  })

  return (
    <div className="grid grid-cols-2 divide-x">
      <DualPanel
        label="Remaining"
        elements={remainingElements}
        isActive={isLineModeActive && activePanel === "remaining"}
        onRowMouseDown={onRowMouseDown("remaining")}
        onRowMouseEnter={
          activePanel === "remaining" ? drag.onRowMouseEnter : undefined
        }
        onRowMouseUp={
          activePanel === "remaining" ? drag.onRowMouseUp : undefined
        }
        selectedRange={selection.selectionRange}
        cursor={selection.state?.cursor ?? null}
      />
      <DualPanel
        label="Reviewed"
        elements={reviewedElements}
        isActive={isLineModeActive && activePanel === "reviewed"}
        onRowMouseDown={onRowMouseDown("reviewed")}
        onRowMouseEnter={
          activePanel === "reviewed" ? drag.onRowMouseEnter : undefined
        }
        onRowMouseUp={
          activePanel === "reviewed" ? drag.onRowMouseUp : undefined
        }
        selectedRange={selection.selectionRange}
        cursor={selection.state?.cursor ?? null}
      />
    </div>
  )
}

function DualPanel({
  label,
  elements,
  isActive,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
  cursor,
  selectedRange,
}: {
  label: string
  elements: DiffElement[]
  isActive?: boolean
  onRowMouseDown?: (line: DiffLine) => void
  onRowMouseEnter?: (line: DiffLine) => void
  onRowMouseUp?: () => void
  cursor: CursorPosition | null
  selectedRange: SelectionRange
}) {
  const hunkElements = elements.flatMap((el, idx) =>
    el.type === "hunk" ? [{ element: el, originalIndex: idx }] : [],
  )

  return (
    <div
      className={cn(
        "bg-background",
        isActive && "ring-2 ring-inset ring-blue-400 dark:ring-blue-600",
      )}
    >
      <div className="px-3 py-1 text-xs font-medium text-muted-foreground bg-muted/50 border-b">
        {label}
      </div>
      {hunkElements.length === 0 ? (
        <div className="p-4 text-center text-muted-foreground text-sm">
          No changes
        </div>
      ) : (
        hunkElements.map(({ element, originalIndex }) => (
          <div key={`hunk-${originalIndex}`}>
            <div className="px-3 py-0.5 font-mono text-xs text-muted-foreground bg-muted/30 border-y border-border/50 select-none">
              {element.hunk.header}
            </div>
            <UnifiedHunkLines
              hunk={element.hunk}
              onRowMouseDown={onRowMouseDown}
              onRowMouseEnter={onRowMouseEnter}
              onRowMouseUp={onRowMouseUp}
              cursor={isActive ? cursor : null}
              selectedRange={
                isActive ? selectedRange : { left: null, right: null }
              }
            />
          </div>
        ))
      )}
    </div>
  )
}
