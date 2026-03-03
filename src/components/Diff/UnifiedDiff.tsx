import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { changedTokenBg, getLineStyle } from "./diffStyles"
import { GapRow } from "./GapRow"
import { InlineThreadDisplay } from "./InlineThreadDisplay"
import { LineNumberGutter } from "./LineNumberGutter"
import { DiffViewProps } from "./SplitDiff"
import {
  CommentContext,
  CommentLineState,
  inlineCommentsKey,
  InlineCommentsMap,
} from "./types"
import {
  getLineHighlightBg,
  LineNavProps,
  SelectionHighlightProps,
} from "./useLineSelection"

export function UnifiedDiff(props: DiffViewProps) {
  const { elements, onExpandGap, selectionHighlight, ...rest } = props

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
          <UnifiedHunkLines
            key={`hunk-${idx}`}
            hunk={el.hunk}
            elementIndex={idx}
            selectionHighlight={selectionHighlight}
            {...rest}
          />
        ),
      )}
    </div>
  )
}

type HunkLinesProps = {
  hunk: DiffHunk
  elementIndex: number
  commentLine?: CommentLineState
  onRowMouseDown?: (globalIndex: number) => void
  onRowMouseEnter?: (globalIndex: number) => void
  onRowMouseUp?: () => void
  commentForm?: React.ReactNode
  selectionHighlight?: SelectionHighlightProps
  inlineComments?: InlineCommentsMap
  commentContext?: CommentContext
}

export function UnifiedHunkLines({
  hunk,
  elementIndex,
  commentLine,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
  commentForm,
  selectionHighlight,
  inlineComments,
  commentContext,
}: HunkLinesProps) {
  const isCommentTarget = (line: DiffLine): boolean => {
    const lineNumber =
      line.lineType === "deletion"
        ? line.oldLineno
        : (line.newLineno ?? line.oldLineno)
    const side: "LEFT" | "RIGHT" =
      line.lineType === "deletion" ? "LEFT" : "RIGHT"

    return commentLine?.line === lineNumber && commentLine.side === side
  }

  const isInCommentRange = (line: DiffLine): boolean => {
    if (!commentLine?.startLine) return false
    const lineNumber =
      line.lineType === "deletion"
        ? line.oldLineno
        : (line.newLineno ?? line.oldLineno)
    const side: "LEFT" | "RIGHT" =
      line.lineType === "deletion" ? "LEFT" : "RIGHT"
    if (side !== commentLine.side) return false
    return (
      lineNumber != null &&
      lineNumber >= commentLine.startLine &&
      lineNumber <= commentLine.line
    )
  }

  const key = (line: DiffLine) =>
    line.lineType === "deletion"
      ? `old-${line.oldLineno}`
      : `new-${line.newLineno ?? line.oldLineno}`

  const baseOffset =
    selectionHighlight?.elementRowOffsets.get(elementIndex) ?? 0

  const lineHeight = 20
  return (
    <div
      className="font-mono text-xs"
      style={{
        contentVisibility: "auto",
        containIntrinsicSize: `auto ${hunk.lines.length * lineHeight}px`,
      }}
    >
      {hunk.lines.map((line, lineIdx) => {
        const globalIndex = baseOffset + lineIdx
        const lineNav: LineNavProps | undefined = selectionHighlight
          ? {
              navIndex: globalIndex,
              isCursor: globalIndex === selectionHighlight.cursorIndex,
              isSelected:
                selectionHighlight.selectionRange != null &&
                globalIndex >= selectionHighlight.selectionRange.start &&
                globalIndex <= selectionHighlight.selectionRange.end,
            }
          : undefined

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

        return (
          <Fragment key={key(line)}>
            <DiffLineComponent
              line={line}
              onRowMouseDown={
                onRowMouseDown ? () => onRowMouseDown(globalIndex) : undefined
              }
              onRowMouseEnter={
                onRowMouseEnter ? () => onRowMouseEnter(globalIndex) : undefined
              }
              onRowMouseUp={onRowMouseUp}
              isInRange={isInCommentRange(line)}
              hasComments={threads.length > 0}
              lineNav={lineNav}
            />
            {threads.length > 0 && (
              <div className="border-y border-border bg-muted/20 px-4 py-2 space-y-2">
                {threads.map((thread) => (
                  <InlineThreadDisplay
                    key={thread.id}
                    thread={thread}
                    onReply={commentContext?.onReplyToThread}
                  />
                ))}
              </div>
            )}
            {isCommentTarget(line) && commentForm && (
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
  hasComments,
  lineNav,
}: {
  line: DiffLine
  onRowMouseDown?: () => void
  onRowMouseEnter?: () => void
  onRowMouseUp?: () => void
  isInRange?: boolean
  hasComments?: boolean
  lineNav?: LineNavProps
}) {
  const { bgColor } = getLineStyle(line.lineType)

  const lineNumber =
    line.lineType === "deletion"
      ? line.oldLineno
      : (line.newLineno ?? line.oldLineno)

  const lineBg = getLineHighlightBg({
    isCursor: lineNav?.isCursor,
    isSelected: lineNav?.isSelected || isInRange,
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
      data-nav-index={lineNav?.navIndex}
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
