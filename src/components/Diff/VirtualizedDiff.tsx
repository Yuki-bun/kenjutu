import { Virtualizer } from "@tanstack/react-virtual"

import { GapRow } from "./GapRow"
import { HunkGap } from "./hunkGaps"
import { ExpandDirection, SplitLineRow } from "./SplitDiff"
import { CommentLineState } from "./types"
import { DiffLineComponent } from "./UnifiedDiff"
import { LineCursorProps, LineNavProps } from "./useLineMode"
import { VirtualRow, VirtualRowModel } from "./virtualRows"

type VirtualizedDiffProps = {
  rowModel: VirtualRowModel
  virtualizer: Virtualizer<HTMLDivElement, Element>
  onExpandGap: (gap: HunkGap, direction: ExpandDirection) => void
  commentLine?: CommentLineState
  onLineDragStart?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnter?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnd?: () => void
  commentForm?: React.ReactNode
  lineCursor?: LineCursorProps
}

function isInCommentRange(
  commentLine: CommentLineState,
  lineNumber: number | null,
  side: "LEFT" | "RIGHT",
): boolean {
  if (!commentLine?.startLine || lineNumber == null) return false
  if (side !== commentLine.side) return false
  return lineNumber >= commentLine.startLine && lineNumber <= commentLine.line
}

function getLineNav(
  navIndex: number,
  lineCursor?: LineCursorProps,
): LineNavProps | undefined {
  if (!lineCursor) return undefined
  return {
    navIndex,
    isCursor: navIndex === lineCursor.cursorIndex,
    isSelected: !!lineCursor.selectedIndices.has(navIndex),
  }
}

function RenderRow({
  row,
  props,
}: {
  row: VirtualRow
  props: VirtualizedDiffProps
}) {
  const {
    onExpandGap,
    commentLine,
    onLineDragStart,
    onLineDragEnter,
    onLineDragEnd,
    commentForm,
    lineCursor,
  } = props

  switch (row.type) {
    case "gap":
      return (
        <GapRow gap={row.gap} isLast={row.isLast} onExpandGap={onExpandGap} />
      )
    case "unifiedLine": {
      const line = row.line
      const lineNumber =
        line.lineType === "deletion"
          ? line.oldLineno
          : (line.newLineno ?? line.oldLineno)
      const side: "LEFT" | "RIGHT" =
        line.lineType === "deletion" ? "LEFT" : "RIGHT"
      return (
        <DiffLineComponent
          line={line}
          onLineDragStart={onLineDragStart}
          onLineDragEnter={onLineDragEnter}
          onLineDragEnd={onLineDragEnd}
          isInRange={isInCommentRange(commentLine ?? null, lineNumber, side)}
          lineNav={getLineNav(row.navIndex, lineCursor)}
        />
      )
    }
    case "splitLine": {
      const pair = row.pair
      const leftInRange = isInCommentRange(
        commentLine ?? null,
        pair.left?.oldLineno ?? null,
        "LEFT",
      )
      const rightInRange = isInCommentRange(
        commentLine ?? null,
        pair.right?.newLineno ?? null,
        "RIGHT",
      )
      return (
        <SplitLineRow
          pair={pair}
          onLineDragStart={onLineDragStart}
          onLineDragEnter={onLineDragEnter}
          onLineDragEnd={onLineDragEnd}
          leftInRange={leftInRange}
          rightInRange={rightInRange}
          lineNav={getLineNav(row.navIndex, lineCursor)}
        />
      )
    }
    case "commentForm":
      return (
        <div className="border-y border-blue-300 dark:border-blue-700 bg-muted/30">
          {commentForm}
        </div>
      )
  }
}

export function VirtualizedDiff(props: VirtualizedDiffProps) {
  const { virtualizer, rowModel } = props
  const virtualItems = virtualizer.getVirtualItems()

  return (
    <div
      className="relative w-full"
      style={{ height: `${virtualizer.getTotalSize()}px` }}
    >
      {virtualItems.map((virtualItem) => {
        const row = rowModel.rows[virtualItem.index]
        return (
          <div
            key={virtualItem.key}
            ref={virtualizer.measureElement}
            data-index={virtualItem.index}
            className="absolute top-0 left-0 w-full"
            style={{
              transform: `translateY(${virtualItem.start - virtualizer.options.scrollMargin}px)`,
            }}
          >
            <RenderRow row={row} props={props} />
          </div>
        )
      })}
    </div>
  )
}
