import { useHotkey } from "@tanstack/react-hotkeys"
import { useState } from "react"

import { HunkId } from "@/bindings"
import { cn } from "@/lib/utils"

import { DiffElement } from "./hunkGaps"
import { UnifiedHunkLines } from "./UnifiedDiff"
import { LineCursorProps, LineModeControl, useLineMode } from "./useLineMode"

export type DualDiffPanel = "remaining" | "reviewed"

type DualDiffProps = {
  remainingElements: DiffElement[]
  reviewedElements: DiffElement[]
  lineMode?: LineModeControl
  onMarkRegion?: (region: HunkId, panel: DualDiffPanel) => void
  fileItemRef: React.RefObject<HTMLDivElement | null>
}

export function DualDiff({
  remainingElements,
  reviewedElements,
  lineMode,
  onMarkRegion,
  fileItemRef,
}: DualDiffProps) {
  const [activePanel, setActivePanel] = useState<DualDiffPanel>("remaining")

  const isLineModeActive =
    lineMode?.state !== null && lineMode?.state !== undefined

  useHotkey(
    "Tab",
    () => {
      setActivePanel((prev) => {
        const next = prev === "remaining" ? "reviewed" : "remaining"
        lineMode?.setState({
          cursorIndex: 0,
          selection: { isSelecting: false },
        })
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

  const { lineCursor } = useLineMode({
    elements: activeElements,
    diffViewMode: "unified",
    containerRef: fileItemRef,
    state: lineMode?.state ?? null,
    setState: lineMode?.setState ?? (() => {}),
    onExit: lineMode?.onExit ?? (() => {}),
    onMarkRegion: handleMarkRegionForPanel,
  })

  return (
    <div className="grid grid-cols-2 divide-x">
      <DualPanel
        label="Remaining"
        elements={remainingElements}
        isActive={isLineModeActive && activePanel === "remaining"}
        lineCursor={activePanel === "remaining" ? lineCursor : undefined}
      />
      <DualPanel
        label="Reviewed"
        elements={reviewedElements}
        isActive={isLineModeActive && activePanel === "reviewed"}
        lineCursor={activePanel === "reviewed" ? lineCursor : undefined}
      />
    </div>
  )
}

function DualPanel({
  label,
  elements,
  isActive,
  lineCursor,
}: {
  label: string
  elements: DiffElement[]
  isActive?: boolean
  lineCursor?: LineCursorProps
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
              lineCursor={lineCursor}
            />
          </div>
        ))
      )}
    </div>
  )
}
