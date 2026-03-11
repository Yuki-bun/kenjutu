import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { changedTokenBg, getLineStyle } from "./diffStyles"
import { GapRow } from "./GapRow"
import { DiffElement, HunkGap } from "./hunkGaps"
import { InlineThreadDisplay } from "./InlineThreadDisplay"
import { LineNumberGutter } from "./LineNumberGutter"
import { PairedLine, pairLinesForSplitView } from "./splitViewPairing"
import { CommentContext, inlineCommentsKey, InlineCommentsMap } from "./types"
import {
  CursorPosition,
  getLineHighlightBg,
  SelectionRange,
} from "./useLineSelection"

export type ExpandDirection = "up" | "down" | "all"

export type DiffViewProps = {
  elements: DiffElement[]
  onExpandGap: (gap: HunkGap, direction: ExpandDirection) => void
  onRowMouseDown?: (line: DiffLine) => void
  onRowMouseEnter?: (line: DiffLine) => void
  onRowMouseUp?: () => void
  commentForm?: React.ReactNode
  inlineComments?: InlineCommentsMap
  commentContext?: CommentContext
  cursor: CursorPosition | null
  selectedRange: SelectionRange
}

export function SplitDiff(props: DiffViewProps) {
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
          <SplitHunkLines key={`hunk-${idx}`} hunk={el.hunk} {...rest} />
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

function SplitHunkLines({
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
  const pairedLines = pairLinesForSplitView(hunk.lines)
  const isPairCursorLine = (pair: PairedLine): boolean => {
    if (!cursor) return false
    return (
      (cursor.side === "LEFT" && pair.left?.oldLineno === cursor.line) ||
      (cursor.side === "RIGHT" && pair.right?.newLineno === cursor.line)
    )
  }

  const isPairInRange = (
    pair: PairedLine,
  ): { left: boolean; right: boolean } => {
    const leftInRange = pair.left
      ? selectedRange.left != null &&
        pair.left.oldLineno != null &&
        selectedRange.left.start <= pair.left.oldLineno &&
        pair.left.oldLineno <= selectedRange.left.end
      : false

    const rightInRange = pair.right
      ? selectedRange.right != null &&
        pair.right.newLineno != null &&
        selectedRange.right.start <= pair.right.newLineno &&
        pair.right.newLineno <= selectedRange.right.end
      : false

    return { left: leftInRange, right: rightInRange }
  }

  const key = (pair: PairedLine) =>
    pair.left?.oldLineno
      ? `L${pair.left.oldLineno}`
      : `R${pair.right?.newLineno}`

  const lineHeight = 20
  return (
    <div
      className="font-mono text-xs"
      style={{
        contentVisibility: "auto",
        containIntrinsicSize: `auto ${pairedLines.length * lineHeight}px`,
      }}
    >
      {pairedLines.map((pair) => {
        const leftThreads =
          pair.left?.oldLineno != null
            ? (inlineComments?.get(
                inlineCommentsKey("LEFT", pair.left.oldLineno),
              ) ?? [])
            : []
        const rightThreads =
          pair.right?.newLineno != null
            ? (inlineComments?.get(
                inlineCommentsKey("RIGHT", pair.right.newLineno),
              ) ?? [])
            : []
        const hasThreads = leftThreads.length > 0 || rightThreads.length > 0
        const line = pair.left ?? pair.right
        const isCursor = isPairCursorLine(pair)

        return (
          <Fragment key={key(pair)}>
            <SplitLineRow
              pair={pair}
              onRowMouseDown={
                onRowMouseDown && line ? () => onRowMouseDown(line) : undefined
              }
              onRowMouseEnter={
                onRowMouseEnter && line
                  ? () => onRowMouseEnter(line)
                  : undefined
              }
              onRowMouseUp={onRowMouseUp}
              leftInRange={isPairInRange(pair).left}
              rightInRange={isPairInRange(pair).right}
              leftHasComments={leftThreads.length > 0}
              rightHasComments={rightThreads.length > 0}
              isCursor={isCursor}
            />
            {(hasThreads || (isCursor && commentForm)) && (
              <div className="flex border-y border-border">
                <div className="flex-1 min-w-0 border-r border-border">
                  {leftThreads.length > 0 && (
                    <div className="bg-muted/20 px-4 py-2 space-y-2">
                      {leftThreads.map((thread) => (
                        <InlineThreadDisplay
                          key={thread.id}
                          thread={thread}
                          onReply={commentContext?.onReplyToThread}
                        />
                      ))}
                    </div>
                  )}
                  {isCursor && commentForm && cursor?.side === "LEFT" && (
                    <div className="border-t border-blue-300 dark:border-blue-700 bg-muted/30">
                      {commentForm}
                    </div>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  {rightThreads.length > 0 && (
                    <div className="bg-muted/20 px-4 py-2 space-y-2">
                      {rightThreads.map((thread) => (
                        <InlineThreadDisplay
                          key={thread.id}
                          thread={thread}
                          onReply={commentContext?.onReplyToThread}
                        />
                      ))}
                    </div>
                  )}
                  {isCursor && commentForm && cursor?.side === "RIGHT" && (
                    <div className="border-t border-blue-300 dark:border-blue-700 bg-muted/30">
                      {commentForm}
                    </div>
                  )}
                </div>
              </div>
            )}
          </Fragment>
        )
      })}
    </div>
  )
}

function SplitLineRow({
  pair,
  onRowMouseDown,
  onRowMouseEnter,
  onRowMouseUp,
  leftInRange,
  rightInRange,
  leftHasComments,
  rightHasComments,
  isCursor,
}: {
  pair: PairedLine
  onRowMouseDown?: () => void
  onRowMouseEnter?: () => void
  onRowMouseUp?: () => void
  leftInRange: boolean
  rightInRange: boolean
  leftHasComments?: boolean
  rightHasComments?: boolean
  isCursor: boolean
}) {
  const defaultLeftBg = pair.left
    ? getLineStyle(pair.left.lineType).bgColor
    : "bg-muted/30"

  const defaultRightBg = pair.right
    ? getLineStyle(pair.right.lineType).bgColor
    : "bg-muted/30"

  // TODO: maybe have to handle each side differently
  const leftBg = getLineHighlightBg({
    isCursor,
    isSelected: leftInRange,
    defaultBg: defaultLeftBg,
  })

  const rightBg = getLineHighlightBg({
    isCursor,
    isSelected: rightInRange,
    defaultBg: defaultRightBg,
  })

  return (
    <div
      className={cn("flex", onRowMouseDown && "cursor-pointer")}
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
      {/* Left side (old file) */}
      <div
        className={cn(
          "flex flex-1 min-w-0 border-r border-border group/line relative",
          leftBg,
        )}
      >
        <LineNumberGutter className="w-10" hasComments={leftHasComments}>
          {pair.left?.oldLineno ?? ""}
        </LineNumberGutter>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.left
            ? pair.left.tokens.map((token, idx) => (
                <span
                  key={idx}
                  style={{ color: token.color ?? undefined }}
                  className={cn(
                    token.changed &&
                      pair.left?.lineType === "deletion" &&
                      changedTokenBg.deletion,
                  )}
                >
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>

      {/* Right side (new file) */}
      <div className={cn("flex flex-1 min-w-0 group/line relative", rightBg)}>
        <LineNumberGutter className="w-10" hasComments={rightHasComments}>
          {pair.right?.newLineno ?? ""}
        </LineNumberGutter>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.right
            ? pair.right.tokens.map((token, idx) => (
                <span
                  key={idx}
                  style={{ color: token.color ?? undefined }}
                  className={cn(
                    token.changed &&
                      pair.right?.lineType === "addition" &&
                      changedTokenBg.addition,
                  )}
                >
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>
    </div>
  )
}
