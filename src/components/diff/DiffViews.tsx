import { DiffHunk, DiffLine } from "@/bindings"
import { cn } from "@/lib/utils"

import { getLineStyle } from "./diffStyles"

export function UnifiedDiffView({ hunks }: { hunks: DiffHunk[] }) {
  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => (
        <div key={idx}>
          {/* Hunk Header */}
          <div className="bg-blue-50 dark:bg-blue-950 px-2 py-1 text-xs font-mono text-blue-700 dark:text-blue-300">
            {hunk.header}
          </div>

          {/* Hunk Lines */}
          <div className="font-mono text-xs">
            {hunk.lines.map((line, lineIdx) => (
              <DiffLineComponent key={lineIdx} line={line} />
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

export function SplitDiffView({ hunks }: { hunks: DiffHunk[] }) {
  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => {
        const pairedLines = pairLinesForSplitView(hunk.lines)
        return (
          <div key={idx}>
            {/* Hunk Header */}
            <div className="bg-blue-50 dark:bg-blue-950 px-2 py-1 text-xs font-mono text-blue-700 dark:text-blue-300">
              {hunk.header}
            </div>

            {/* Hunk Lines - Split View */}
            <div className="font-mono text-xs">
              {pairedLines.map((pair, lineIdx) => (
                <SplitLineRow key={lineIdx} pair={pair} />
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}

type PairedLine = {
  left: DiffLine | null
  right: DiffLine | null
}

function pairLinesForSplitView(lines: DiffLine[]): PairedLine[] {
  const result: PairedLine[] = []
  let i = 0

  while (i < lines.length) {
    const line = lines[i]

    if (
      line.lineType === "context" ||
      line.lineType === "addeofnl" ||
      line.lineType === "deleofnl"
    ) {
      // Context lines appear on both sides
      result.push({ left: line, right: line })
      i++
    } else if (line.lineType === "deletion") {
      // Collect consecutive deletions
      const deletions: DiffLine[] = []
      while (i < lines.length && lines[i].lineType === "deletion") {
        deletions.push(lines[i])
        i++
      }

      // Collect following consecutive additions
      const additions: DiffLine[] = []
      while (i < lines.length && lines[i].lineType === "addition") {
        additions.push(lines[i])
        i++
      }

      // Pair them up side-by-side
      const maxLen = Math.max(deletions.length, additions.length)
      for (let j = 0; j < maxLen; j++) {
        result.push({
          left: deletions[j] ?? null,
          right: additions[j] ?? null,
        })
      }
    } else if (line.lineType === "addition") {
      // Standalone addition (no preceding deletion)
      result.push({ left: null, right: line })
      i++
    } else {
      i++
    }
  }

  return result
}

function SplitLineRow({ pair }: { pair: PairedLine }) {
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
      <div className={cn("flex flex-1 min-w-0 border-r border-border", leftBg)}>
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0">
          {pair.left?.oldLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.left
            ? pair.left.tokens.map((token, idx) => (
                <span key={idx} style={{ color: token.color ?? undefined }}>
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>

      {/* Right side (new file) */}
      <div className={cn("flex flex-1 min-w-0", rightBg)}>
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0">
          {pair.right?.newLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.right
            ? pair.right.tokens.map((token, idx) => (
                <span key={idx} style={{ color: token.color ?? undefined }}>
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>
    </div>
  )
}

function DiffLineComponent({ line }: { line: DiffLine }) {
  const { bgColor } = getLineStyle(line.lineType)

  return (
    <div className={cn("flex hover:bg-muted/30", bgColor)}>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0">
        {line.oldLineno || ""}
      </span>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0">
        {line.newLineno || ""}
      </span>
      <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word">
        {line.tokens.map((token, idx) => (
          <span key={idx} style={{ color: token.color ?? undefined }}>
            {token.content}
          </span>
        ))}
      </span>
    </div>
  )
}
