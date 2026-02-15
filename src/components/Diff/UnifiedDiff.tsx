import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { getLineStyle } from "./diffStyles"
import { CommentLineState } from "./FileDiffItem"
import { GapRow } from "./GapRow"
import { LineCommentButton } from "./LineCommentButton"
import { DiffViewProps } from "./SplitDiff"

export function UnifiedDiff(props: DiffViewProps) {
  const { elements, onExpandGap } = props

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
          <HunkLines key={`hunk-${idx}`} hunk={el.hunk} {...props} />
        ),
      )}
    </div>
  )
}

type HunkLinesProps = {
  hunk: DiffHunk
  commentLine?: CommentLineState
  onLineComment?: (line: number, side: "LEFT" | "RIGHT") => void
  commentForm?: React.ReactNode
}

function HunkLines({
  hunk,
  commentLine,
  onLineComment,
  commentForm,
}: HunkLinesProps) {
  const isCommentTarget = (line: DiffLine): boolean => {
    const lineNumber =
      line.lineType === "deletion"
        ? line.oldLineno
        : (line.newLineno ?? line.oldLineno)
    const side: "LEFT" | "RIGHT" =
      line.lineType === "deletion" ? "LEFT" : "RIGHT"

    return commentLine?.line === lineNumber && commentLine?.side === side
  }

  return (
    <div className="font-mono text-xs">
      {hunk.lines.map((line) => (
        <Fragment key={line.newLineno || line.oldLineno}>
          <DiffLineComponent line={line} onLineComment={onLineComment} />
          {isCommentTarget(line) && commentForm && (
            <div className="border-y border-blue-300 dark:border-blue-700 bg-muted/30">
              {commentForm}
            </div>
          )}
        </Fragment>
      ))}
    </div>
  )
}

function DiffLineComponent({
  line,
  onLineComment,
}: {
  line: DiffLine
  onLineComment?: (line: number, side: "LEFT" | "RIGHT") => void
}) {
  const { bgColor } = getLineStyle(line.lineType)

  const lineNumber =
    line.lineType === "deletion"
      ? line.oldLineno
      : (line.newLineno ?? line.oldLineno)
  const side: "LEFT" | "RIGHT" = line.lineType === "deletion" ? "LEFT" : "RIGHT"

  const showButtonOnOld = line.lineType === "deletion" && line.oldLineno != null
  const showButtonOnNew = line.lineType !== "deletion" && line.newLineno != null

  return (
    <div className={cn("flex hover:bg-muted/30 group/line relative", bgColor)}>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0 relative">
        {onLineComment && showButtonOnOld && (
          <LineCommentButton
            onClick={() => onLineComment(line.oldLineno!, "LEFT")}
          />
        )}
        {line.lineType !== "addition" && line.oldLineno}
      </span>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0 relative">
        {onLineComment && showButtonOnNew && lineNumber != null && (
          <LineCommentButton onClick={() => onLineComment(lineNumber, side)} />
        )}
        {line.lineType !== "deletion" && lineNumber}
      </span>
      <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word">
        {line.tokens.map((token, idx) => (
          <span
            key={idx}
            style={{ color: token.color ?? undefined }}
            className={cn(
              token.changed &&
                line.lineType === "deletion" &&
                "bg-red-300/60 dark:bg-red-700/60",
              token.changed &&
                line.lineType === "addition" &&
                "bg-green-300/60 dark:bg-green-700/60",
            )}
          >
            {token.content}
          </span>
        ))}
      </span>
    </div>
  )
}
