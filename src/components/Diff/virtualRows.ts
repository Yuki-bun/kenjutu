import { DiffLine } from "@/bindings"

import { DiffElement, HunkGap } from "./hunkGaps"
import { PairedLine, pairLinesForSplitView } from "./SplitDiff"
import { CommentLineState } from "./types"
import { DiffViewMode } from "./useDiffViewMode"

export type VirtualRow =
  | { type: "gap"; gap: HunkGap; isLast: boolean }
  | { type: "unifiedLine"; line: DiffLine; navIndex: number }
  | { type: "splitLine"; pair: PairedLine; navIndex: number }
  | { type: "commentForm" }

export type VirtualRowModel = {
  rows: VirtualRow[]
  /** navIndex → virtual row index */
  navToVirtual: number[]
  totalNavRows: number
}

function isCommentTarget(
  line: DiffLine,
  commentLine: NonNullable<CommentLineState>,
): boolean {
  const lineNumber =
    line.lineType === "deletion"
      ? line.oldLineno
      : (line.newLineno ?? line.oldLineno)
  const side: "LEFT" | "RIGHT" = line.lineType === "deletion" ? "LEFT" : "RIGHT"
  return commentLine.line === lineNumber && commentLine.side === side
}

function isSplitCommentTarget(
  pair: PairedLine,
  commentLine: NonNullable<CommentLineState>,
): boolean {
  const isLeftTarget =
    pair.left &&
    commentLine.side === "LEFT" &&
    pair.left.oldLineno === commentLine.line
  const isRightTarget =
    pair.right &&
    commentLine.side === "RIGHT" &&
    pair.right.newLineno === commentLine.line
  return !!(isLeftTarget || isRightTarget)
}

export function buildVirtualRowModel(
  elements: DiffElement[],
  diffViewMode: DiffViewMode,
  commentLine: CommentLineState,
): VirtualRowModel {
  const rows: VirtualRow[] = []
  const navToVirtual: number[] = []
  let navIndex = 0

  for (let i = 0; i < elements.length; i++) {
    const el = elements[i]

    if (el.type === "gap") {
      rows.push({
        type: "gap",
        gap: el.gap,
        isLast: i === elements.length - 1,
      })
      continue
    }

    if (diffViewMode === "split") {
      const pairs = pairLinesForSplitView(el.hunk.lines)
      for (const pair of pairs) {
        navToVirtual.push(rows.length)
        rows.push({ type: "splitLine", pair, navIndex })
        navIndex++

        if (commentLine && isSplitCommentTarget(pair, commentLine)) {
          rows.push({ type: "commentForm" })
        }
      }
    } else {
      for (const line of el.hunk.lines) {
        navToVirtual.push(rows.length)
        rows.push({ type: "unifiedLine", line, navIndex })
        navIndex++

        if (commentLine && isCommentTarget(line, commentLine)) {
          rows.push({ type: "commentForm" })
        }
      }
    }
  }

  return { rows, navToVirtual, totalNavRows: navIndex }
}
