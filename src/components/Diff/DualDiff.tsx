import { useHotkey } from "@tanstack/react-hotkeys"
import { useState } from "react"

import { HunkId } from "@/bindings"
import { cn } from "@/lib/utils"

import { DiffElement } from "./hunkGaps"
import { UnifiedHunkLines } from "./UnifiedDiff"
import { useLineDrag } from "./useLineDrag"
import { useLineMode } from "./useLineMode"
import {
  LineSelectionControl,
  SelectionHighlightProps,
  useLineSelection,
} from "./useLineSelection"

export type DualDiffPanel = "remaining" | "reviewed"

type DualDiffProps = {
  remainingElements: DiffElement[]
  reviewedElements: DiffElement[]
  lineSelection?: LineSelectionControl
  onMarkRegion?: (region: HunkId, panel: DualDiffPanel) => void
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

  useHotkey(
    "Tab",
    () => {
      setActivePanel((prev) => {
        const next = prev === "remaining" ? "reviewed" : "remaining"
        lineSelection?.setState({ cursorIndex: 0, anchor: null })
        return next
      })
    },
    { enabled: isLineModeActive },
  )

  const activeElements =
    activePanel === "remaining" ? remainingElements : reviewedElements

  const handleMarkRegionForPanel = onMarkRegion
    ? (region: HunkId) => onMarkRegion(region, activePanel)
    : undefined

  const selection = useLineSelection({
    elements: activeElements,
    diffViewMode: "unified",
    state: lineSelection?.state ?? null,
    setState: lineSelection?.setState ?? (() => {}),
  })

  const drag = useLineDrag({
    selection,
    enabled: true,
    onActivate: (globalIndex) => {
      lineSelection?.setState({ cursorIndex: globalIndex, anchor: null })
    },
  })

  const switchAndActivate = (panel: DualDiffPanel, globalIndex: number) => {
    setActivePanel(panel)
    lineSelection?.setState({ cursorIndex: globalIndex, anchor: null })
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
        selectionHighlight={
          activePanel === "remaining" ? selection.highlightProps : undefined
        }
        onRowMouseDown={
          activePanel === "remaining"
            ? drag.onRowMouseDown
            : (globalIndex) => switchAndActivate("remaining", globalIndex)
        }
        onRowMouseEnter={
          activePanel === "remaining" ? drag.onRowMouseEnter : undefined
        }
        onRowMouseUp={
          activePanel === "remaining" ? drag.onRowMouseUp : undefined
        }
      />
      <DualPanel
        label="Reviewed"
        elements={reviewedElements}
        isActive={isLineModeActive && activePanel === "reviewed"}
        selectionHighlight={
          activePanel === "reviewed" ? selection.highlightProps : undefined
        }
        onRowMouseDown={
          activePanel === "reviewed"
            ? drag.onRowMouseDown
            : (globalIndex) => switchAndActivate("reviewed", globalIndex)
        }
        onRowMouseEnter={
          activePanel === "reviewed" ? drag.onRowMouseEnter : undefined
        }
        onRowMouseUp={
          activePanel === "reviewed" ? drag.onRowMouseUp : undefined
        }
      />
    </div>
  )
}

function DualPanel({
  label,
  elements,
  isActive,
  selectionHighlight,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
}: {
  label: string
  elements: DiffElement[]
  isActive?: boolean
  selectionHighlight?: SelectionHighlightProps
  onRowMouseDown?: (globalIndex: number) => void
  onRowMouseEnter?: (globalIndex: number) => void
  onRowMouseUp?: () => void
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
              elementIndex={originalIndex}
              selectionHighlight={selectionHighlight}
              onRowMouseDown={onRowMouseDown}
              onRowMouseEnter={onRowMouseEnter}
              onRowMouseUp={onRowMouseUp}
            />
          </div>
        ))
      )}
    </div>
  )
}
