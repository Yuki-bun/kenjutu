import { MessageSquarePlus } from "lucide-react"
import { Fragment } from "react"

import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { getLineStyle } from "./diffStyles"
import { HunkGap } from "./hunkGaps"
import { GapIndicator } from "./HunkGapSeparator"

export type CommentLineState = {
  line: number
  side: "LEFT" | "RIGHT"
} | null

export type ExpandDirection = "up" | "down" | "all"

type DiffViewProps = {
  hunks: DiffHunk[]
  gaps: HunkGap[]
  onExpandGap: (gapIndex: number, direction: ExpandDirection) => void
  expandingGap: number | null
  commentLine?: CommentLineState
  onLineComment?: (line: number, side: "LEFT" | "RIGHT") => void
  commentForm?: React.ReactNode
}

export function UnifiedDiffView(props: DiffViewProps) {
  const { hunks, gaps, onExpandGap, expandingGap } = props

  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => (
        <Fragment key={idx}>
          <HunkGapRow
            gap={gaps?.[idx]}
            gapIndex={idx}
            expandingGap={expandingGap}
            onExpandGap={onExpandGap}
          />
          <UnifiedHunkLines hunk={hunk} {...props} />
        </Fragment>
      ))}
      <HunkGapRow
        gap={gaps?.[hunks.length]}
        gapIndex={hunks.length}
        isTrailing
        expandingGap={expandingGap}
        onExpandGap={onExpandGap}
      />
    </div>
  )
}

export function SplitDiffView({
  hunks,
  gaps,
  onExpandGap,
  expandingGap,
  ...props
}: DiffViewProps) {
  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => (
        <Fragment key={idx}>
          <HunkGapRow
            gap={gaps?.[idx]}
            gapIndex={idx}
            expandingGap={expandingGap}
            onExpandGap={onExpandGap}
          />
          <SplitHunkLines hunk={hunk} {...props} />
        </Fragment>
      ))}
      <HunkGapRow
        gap={gaps?.[hunks.length]}
        gapIndex={hunks.length}
        isTrailing
        expandingGap={expandingGap}
        onExpandGap={onExpandGap}
      />
    </div>
  )
}

function HunkGapRow({
  gap,
  gapIndex,
  isTrailing,
  expandingGap,
  onExpandGap,
}: {
  gap: HunkGap | undefined
  gapIndex: number
  isTrailing?: boolean
  expandingGap: number | null
  onExpandGap: (gapIndex: number, direction: ExpandDirection) => void
}) {
  const count = gap?.count ?? 0
  return (
    <GapIndicator
      hiddenLineCount={count}
      showExpandUp={!isTrailing && count > 0}
      showExpandDown={count > 0 && (isTrailing || gapIndex > 0)}
      isLoading={expandingGap === gapIndex}
      onExpand={(dir) => onExpandGap?.(gapIndex, dir)}
    />
  )
}

type HunkLinesProps = {
  hunk: DiffHunk
  commentLine?: CommentLineState
  onLineComment?: (line: number, side: "LEFT" | "RIGHT") => void
  commentForm?: React.ReactNode
}

function UnifiedHunkLines({
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

function SplitHunkLines({
  hunk,
  commentLine,
  onLineComment,
  commentForm,
}: HunkLinesProps) {
  const pairedLines = pairLinesForSplitView(hunk.lines)
  const isCommentTarget = (pair: PairedLine): boolean => {
    const leftLineNumber = pair.left?.oldLineno
    const rightLineNumber = pair.right?.newLineno

    const isLeftTarget =
      pair.left &&
      commentLine?.side === "LEFT" &&
      leftLineNumber === commentLine.line

    const isRightTarget =
      pair.right &&
      commentLine?.side === "RIGHT" &&
      rightLineNumber === commentLine.line

    return !!(isLeftTarget || isRightTarget)
  }

  return (
    <div className="font-mono text-xs">
      {pairedLines.map((pair) => (
        <Fragment key={pair.right?.newLineno ?? pair.left?.oldLineno}>
          <SplitLineRow pair={pair} onLineComment={onLineComment} />
          {isCommentTarget(pair) && commentForm && (
            <div className="border-y border-blue-300 dark:border-blue-700 bg-muted/30">
              {commentForm}
            </div>
          )}
        </Fragment>
      ))}
    </div>
  )
}

type PairedLine = {
  left: DiffLine | null
  right: DiffLine | null
}

type ProcessResult = {
  pairs: PairedLine[]
  nextIndex: number
}

function pairLinesForSplitView(lines: DiffLine[]): PairedLine[] {
  return processLines(lines, 0).pairs
}

function processLines(lines: DiffLine[], startIndex: number): ProcessResult {
  if (startIndex >= lines.length) {
    return { pairs: [], nextIndex: startIndex }
  }

  const line = lines[startIndex]

  if (
    line.lineType === "context" ||
    line.lineType === "addeofnl" ||
    line.lineType === "deleofnl"
  ) {
    const contextPair: PairedLine = { left: line, right: line }
    const rest = processLines(lines, startIndex + 1)
    return { pairs: [contextPair, ...rest.pairs], nextIndex: rest.nextIndex }
  }

  if (line.lineType === "deletion") {
    const deletionsResult = collectDeletions(lines, startIndex)
    const additionsResult = collectAdditions(lines, deletionsResult.nextIndex)
    const alignedPairs = alignChangedBlock(
      deletionsResult.lines,
      additionsResult.lines,
    )
    const rest = processLines(lines, additionsResult.nextIndex)
    return {
      pairs: [...alignedPairs, ...rest.pairs],
      nextIndex: rest.nextIndex,
    }
  }

  if (line.lineType === "addition") {
    const additionPair: PairedLine = { left: null, right: line }
    const rest = processLines(lines, startIndex + 1)
    return { pairs: [additionPair, ...rest.pairs], nextIndex: rest.nextIndex }
  }

  // Skip unhandled line types
  return processLines(lines, startIndex + 1)
}

function collectDeletions(
  lines: DiffLine[],
  startIndex: number,
): { lines: DiffLine[]; nextIndex: number } {
  const endIndex = lines
    .slice(startIndex)
    .findIndex((line) => line.lineType !== "deletion")

  const actualEndIndex = endIndex === -1 ? lines.length : startIndex + endIndex

  return {
    lines: lines.slice(startIndex, actualEndIndex),
    nextIndex: actualEndIndex,
  }
}

function collectAdditions(
  lines: DiffLine[],
  startIndex: number,
): { lines: DiffLine[]; nextIndex: number } {
  const endIndex = lines
    .slice(startIndex)
    .findIndex((line) => line.lineType !== "addition")

  const actualEndIndex = endIndex === -1 ? lines.length : startIndex + endIndex

  return {
    lines: lines.slice(startIndex, actualEndIndex),
    nextIndex: actualEndIndex,
  }
}

/** Align a block of consecutive deletions + additions using backend match info. */
function alignChangedBlock(
  deletions: DiffLine[],
  additions: DiffLine[],
): PairedLine[] {
  const addByNewLineno = buildAdditionMap(additions)
  const matchPairs = findMatchPairs(deletions, addByNewLineno)

  if (matchPairs.length === 0) {
    return positionalPairing(deletions, additions)
  }

  return alignWithMatchPairs(deletions, additions, matchPairs)
}

function buildAdditionMap(additions: DiffLine[]): Map<number, number> {
  return additions.reduce((map, addition, idx) => {
    if (addition.newLineno != null) {
      map.set(addition.newLineno, idx)
    }
    return map
  }, new Map<number, number>())
}

function findMatchPairs(
  deletions: DiffLine[],
  addByNewLineno: Map<number, number>,
): [number, number][] {
  return deletions.reduce<[number, number][]>((pairs, deletion, delIdx) => {
    if (deletion.newLineno != null) {
      const addIdx = addByNewLineno.get(deletion.newLineno)
      if (addIdx != null) {
        return [...pairs, [delIdx, addIdx]]
      }
    }
    return pairs
  }, [])
}

function positionalPairing(
  deletions: DiffLine[],
  additions: DiffLine[],
): PairedLine[] {
  const maxLen = Math.max(deletions.length, additions.length)
  return Array.from({ length: maxLen }, (_, j) => ({
    left: deletions[j] ?? null,
    right: additions[j] ?? null,
  }))
}

function alignWithMatchPairs(
  deletions: DiffLine[],
  additions: DiffLine[],
  matchPairs: [number, number][],
): PairedLine[] {
  type AlignState = {
    pairs: PairedLine[]
    delPtr: number
    addPtr: number
  }

  const finalState = matchPairs.reduce<AlignState>(
    (state, [delIdx, addIdx]) => {
      // Emit unmatched deletions before this pair
      const unmatchedDels = createRange(state.delPtr, delIdx).map((i) => ({
        left: deletions[i],
        right: null,
      }))

      // Emit unmatched additions before this pair
      const unmatchedAdds = createRange(state.addPtr, addIdx).map((i) => ({
        left: null,
        right: additions[i],
      }))

      // Emit the matched pair
      const matchedPair: PairedLine = {
        left: deletions[delIdx],
        right: additions[addIdx],
      }

      return {
        pairs: [
          ...state.pairs,
          ...unmatchedDels,
          ...unmatchedAdds,
          matchedPair,
        ],
        delPtr: delIdx + 1,
        addPtr: addIdx + 1,
      }
    },
    { pairs: [], delPtr: 0, addPtr: 0 },
  )

  // Remaining unmatched lines after all pairs
  const remainingDels = createRange(finalState.delPtr, deletions.length).map(
    (i) => ({
      left: deletions[i],
      right: null,
    }),
  )

  const remainingAdds = createRange(finalState.addPtr, additions.length).map(
    (i) => ({
      left: null,
      right: additions[i],
    }),
  )

  return [...finalState.pairs, ...remainingDels, ...remainingAdds]
}

function createRange(start: number, end: number): number[] {
  return Array.from({ length: Math.max(0, end - start) }, (_, i) => start + i)
}

function LineCommentButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      className="absolute left-0 top-1/2 -translate-y-1/2 opacity-0 group-hover/line:opacity-100 transition-opacity bg-blue-500 text-white rounded-sm p-0.5 hover:bg-blue-600 z-10"
    >
      <MessageSquarePlus className="w-3 h-3" />
    </button>
  )
}

function SplitLineRow({
  pair,
  onLineComment,
}: {
  pair: PairedLine
  onLineComment?: (line: number, side: "LEFT" | "RIGHT") => void
}) {
  const leftBg = pair.left
    ? pair.left.lineType === "deletion"
      ? "bg-red-50 dark:bg-red-950/30"
      : "bg-background"
    : "bg-muted/30"

  const rightBg = pair.right
    ? pair.right.lineType === "addition"
      ? "bg-green-50 dark:bg-green-950/30"
      : "bg-background"
    : "bg-muted/30"

  return (
    <div className="flex">
      {/* Left side (old file) */}
      <div
        className={cn(
          "flex flex-1 min-w-0 border-r border-border group/line relative",
          leftBg,
        )}
      >
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0 relative">
          {onLineComment && pair.left?.oldLineno != null && (
            <LineCommentButton
              onClick={() => onLineComment(pair.left!.oldLineno!, "LEFT")}
            />
          )}
          {pair.left?.oldLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.left
            ? pair.left.tokens.map((token, idx) => (
                <span
                  key={idx}
                  style={{ color: token.color ?? undefined }}
                  className={cn(
                    token.changed &&
                      pair.left?.lineType === "deletion" &&
                      "bg-red-300/60 dark:bg-red-700/60",
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
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0 relative">
          {onLineComment && pair.right?.newLineno != null && (
            <LineCommentButton
              onClick={() => onLineComment(pair.right!.newLineno!, "RIGHT")}
            />
          )}
          {pair.right?.newLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.right
            ? pair.right.tokens.map((token, idx) => (
                <span
                  key={idx}
                  style={{ color: token.color ?? undefined }}
                  className={cn(
                    token.changed &&
                      pair.right?.lineType === "addition" &&
                      "bg-green-300/60 dark:bg-green-700/60",
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
        {line.oldLineno || ""}
      </span>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0 relative">
        {onLineComment && showButtonOnNew && lineNumber != null && (
          <LineCommentButton onClick={() => onLineComment(lineNumber, side)} />
        )}
        {line.newLineno || ""}
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
