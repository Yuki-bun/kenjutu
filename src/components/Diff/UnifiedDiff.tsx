import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { changedTokenBg, getLineStyle } from "./diffStyles"
import { GapRow } from "./GapRow"
import { InlineThreadDisplay } from "./InlineThreadDisplay"
import { LineNumberGutter } from "./LineNumberGutter"
import { DiffViewProps } from "./SplitDiff"
import { CommentContext, inlineCommentsKey, InlineCommentsMap } from "./types"
import {
  CursorPosition,
  diffLineToCursorPosition,
  getLineHighlightBg,
  SelectionRange,
} from "./useLineSelection"

export function UnifiedDiff(props: DiffViewProps) {
  const { elements, onExpandGap, ...rest } = props

  return (
    <div className="bg-background">
      {elements.map((el, idx) =>
        el.type === "gap" ? (
          <GapRow
            key={`gap-${idx}`}
            gap={el.gap}
            isLast={idx === elements.length - 1}
            onExpandGap={onExpandGap}
          />
        ) : (
          <UnifiedHunkLines key={`hunk-${idx}`} hunk={el.hunk} {...rest} />
        ),
      )}
    </div>
  )
}

type HunkLinesProps = {
  hunk: DiffHunk
  onRowMouseDown?: (line: DiffLine) => void
  onRowMouseEnter?: (line: DiffLine) => void
  onRowMouseUp?: () => void
  commentForm?: React.ReactNode
  inlineComments?: InlineCommentsMap
  commentContext?: CommentContext
  cursor: CursorPosition | null
  selectedRange: SelectionRange
}

export function UnifiedHunkLines({
  hunk,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
  commentForm,
  inlineComments,
  commentContext,
  cursor,
  selectedRange,
}: HunkLinesProps) {
  const key = (line: DiffLine) =>
    line.lineType === "deletion"
      ? `old-${line.oldLineno}`
      : `new-${line.newLineno ?? line.oldLineno}`

  const lineHeight = 20

  const isInRange = (line: DiffLine) => {
    const pos = diffLineToCursorPosition(line)
    return pos.side === "LEFT"
      ? selectedRange.left != null &&
          selectedRange.left.start <= pos.line &&
          pos.line <= selectedRange.left.end
      : selectedRange.right != null &&
          selectedRange.right.start <= pos.line &&
          pos.line <= selectedRange.right.end
  }

  const isCursor = (line: DiffLine) => {
    if (!cursor) return false
    const pos = diffLineToCursorPosition(line)
    return pos.side === cursor.side && pos.line === cursor.line
  }

  return (
    <div
      className="font-mono text-xs"
      style={{
        contentVisibility: "auto",
        containIntrinsicSize: `auto ${hunk.lines.length * lineHeight}px`,
      }}
    >
      {hunk.lines.map((line) => {
        const lineNumber =
          line.lineType === "deletion"
            ? line.oldLineno
            : (line.newLineno ?? line.oldLineno)
        const lineSide: "LEFT" | "RIGHT" =
          line.lineType === "deletion" ? "LEFT" : "RIGHT"
        const threads =
          lineNumber != null
            ? (inlineComments?.get(inlineCommentsKey(lineSide, lineNumber)) ??
              [])
            : []
        const unResolvedThreads = threads.filter((thread) => !thread.resolved)

        return (
          <Fragment key={key(line)}>
            <DiffLineComponent
              line={line}
              onRowMouseDown={
                onRowMouseDown ? () => onRowMouseDown(line) : undefined
              }
              onRowMouseEnter={
                onRowMouseEnter ? () => onRowMouseEnter(line) : undefined
              }
              onRowMouseUp={onRowMouseUp}
              isInRange={isInRange(line)}
              isCursor={isCursor(line)}
              hasComments={threads.length > 0}
            />
            {unResolvedThreads.length > 0 && (
              <div className="border-y border-border bg-muted/20 px-4 py-2 space-y-2">
                {unResolvedThreads.map((thread) => (
                  <InlineThreadDisplay
                    key={thread.id}
                    thread={thread}
                    onReply={commentContext?.onReplyToThread}
                  />
                ))}
              </div>
            )}
            {/* TODO: must resolve upper side of selection */}
            {isCursor(line) && commentForm && (
              <div className="border-y border-blue-300 dark:border-blue-700 bg-muted/30">
                {commentForm}
              </div>
            )}
          </Fragment>
        )
      })}
    </div>
  )
}

function DiffLineComponent({
  line,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
  isInRange,
  isCursor,
  hasComments,
}: {
  line: DiffLine
  onRowMouseDown?: () => void
  onRowMouseEnter?: () => void
  onRowMouseUp?: () => void
  isInRange: boolean
  isCursor: boolean
  hasComments?: boolean
}) {
  const { bgColor } = getLineStyle(line.lineType)

  const lineNumber =
    line.lineType === "deletion"
      ? line.oldLineno
      : (line.newLineno ?? line.oldLineno)

  const lineBg = getLineHighlightBg({
    isCursor: isCursor,
    isSelected: isInRange,
    defaultBg: bgColor,
  })

  return (
    <div
      className={cn(
        "flex hover:bg-muted/30 group/line relative",
        lineBg,
        onRowMouseDown && "cursor-pointer",
      )}
      style={{ contain: "content" }}
      data-cursor={isCursor || undefined}
      onMouseDown={
        onRowMouseDown
          ? (e) => {
              e.preventDefault()
              onRowMouseDown()
            }
          : undefined
      }
      onMouseEnter={onRowMouseEnter}
      onMouseUp={onRowMouseUp}
    >
      <LineNumberGutter className="w-12" hasComments={hasComments}>
        {line.lineType !== "addition" && line.oldLineno}
      </LineNumberGutter>
      <LineNumberGutter className="w-12">
        {line.lineType !== "deletion" && lineNumber}
      </LineNumberGutter>
      <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word">
        {line.tokens.map((token, idx) => (
          <span
            key={idx}
            style={{ color: token.color ?? undefined }}
            className={cn(
              token.changed &&
                line.lineType === "deletion" &&
                changedTokenBg.deletion,
              token.changed &&
                line.lineType === "addition" &&
                changedTokenBg.addition,
            )}
          >
            {token.content}
          </span>
        ))}
      </span>
    </div>
  )
}
