import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { changedTokenBg, getLineStyle } from "./diffStyles"
import { GapRow } from "./GapRow"
import { LineNumberGutter } from "./LineNumberGutter"
import { DiffViewProps } from "./SplitDiff"
import { CommentLineState } from "./types"
import {
  getLineHighlightBg,
  LineCursorProps,
  lineIdentityForDiffLine,
  lineKey,
} from "./useLineMode"

export function UnifiedDiff(props: DiffViewProps) {
  const { elements, onExpandGap, lineCursor, ...rest } = props

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
            lineCursor={lineCursor}
            {...rest}
          />
        ),
      )}
    </div>
  )
}

type HunkLinesProps = {
  hunk: DiffHunk
  commentLine?: CommentLineState
  onLineDragStart?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnter?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnd?: () => void
  commentForm?: React.ReactNode
  lineCursor?: LineCursorProps
}

export function UnifiedHunkLines({
  hunk,
  commentLine,
  onLineDragStart,
  onLineDragEnter,
  onLineDragEnd,
  commentForm,
  lineCursor,
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

  const lineHeight = 20
  return (
    <div
      className="font-mono text-xs"
      style={{
        contentVisibility: "auto",
        containIntrinsicSize: `auto ${hunk.lines.length * lineHeight}px`,
      }}
    >
      {hunk.lines.map((line) => {
        const identity = lineIdentityForDiffLine(line)
        const lk = identity ? lineKey(identity) : null
        const isCursor = lk != null && lk === lineCursor?.cursorKey
        const isSelected =
          lk != null && (lineCursor?.selectedKeys.has(lk) ?? false)

        return (
          <Fragment key={key(line)}>
            <DiffLineComponent
              line={line}
              onLineDragStart={onLineDragStart}
              onLineDragEnter={onLineDragEnter}
              onLineDragEnd={onLineDragEnd}
              isInRange={isInCommentRange(line)}
              isCursor={isCursor}
              isSelected={isSelected}
              lineId={lineCursor ? lk : undefined}
            />
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
  onLineDragStart,
  onLineDragEnter,
  onLineDragEnd,
  isInRange,
  isCursor,
  isSelected,
  lineId,
}: {
  line: DiffLine
  onLineDragStart?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnter?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnd?: () => void
  isInRange?: boolean
  isCursor?: boolean
  isSelected?: boolean
  lineId?: string | null
}) {
  const { bgColor } = getLineStyle(line.lineType)

  const lineNumber =
    line.lineType === "deletion"
      ? line.oldLineno
      : (line.newLineno ?? line.oldLineno)
  const side: "LEFT" | "RIGHT" = line.lineType === "deletion" ? "LEFT" : "RIGHT"

  const showButtonOnOld = line.lineType === "deletion" && line.oldLineno != null
  const showButtonOnNew = line.lineType !== "deletion" && line.newLineno != null

  const lineBg = getLineHighlightBg({
    isCursor,
    isSelected,
    isInRange,
    defaultBg: bgColor,
  })

  return (
    <div
      className={cn("flex hover:bg-muted/30 group/line relative", lineBg)}
      style={{ contain: "content" }}
      data-line-id={lineId ?? undefined}
    >
      <LineNumberGutter
        lineNumber={showButtonOnOld ? line.oldLineno! : null}
        side="LEFT"
        className="w-12"
        onLineDragStart={onLineDragStart}
        onLineDragEnter={onLineDragEnter}
        onLineDragEnd={onLineDragEnd}
      >
        {line.lineType !== "addition" && line.oldLineno}
      </LineNumberGutter>
      <LineNumberGutter
        lineNumber={showButtonOnNew && lineNumber != null ? lineNumber : null}
        side={side}
        className="w-12"
        onLineDragStart={onLineDragStart}
        onLineDragEnter={onLineDragEnter}
        onLineDragEnd={onLineDragEnd}
      >
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
