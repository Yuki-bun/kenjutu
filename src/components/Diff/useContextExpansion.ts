import { useCallback, useState } from "react"
import { toast } from "sonner"

import { commands, DiffLine } from "@/bindings"

import { HunkGap } from "./hunkGaps"
import { ExpandDirection } from "./SplitDiff"

const EXPAND_LINES_COUNT = 20

export function useContextExpansion({
  localDir,
  commitSha,
  filePath,
}: {
  localDir: string
  commitSha: string
  filePath: string
}) {
  const [fetchedContextLines, setFetchedContextLines] = useState<
    Map<number, DiffLine>
  >(new Map())

  const handleExpandGap = useCallback(
    async (gap: HunkGap, direction: ExpandDirection) => {
      let fetchStart: number
      let fetchEnd: number

      if (direction === "all") {
        fetchStart = gap.newStart
        fetchEnd = gap.newEnd
      } else if (direction === "down") {
        fetchStart = gap.newStart
        fetchEnd = Math.min(gap.newStart + EXPAND_LINES_COUNT - 1, gap.newEnd)
      } else {
        fetchEnd = gap.newEnd
        fetchStart = Math.max(gap.newEnd - EXPAND_LINES_COUNT + 1, gap.newStart)
      }

      if (fetchStart > fetchEnd) return

      const oldStartLine = gap.oldStart + (fetchStart - gap.newStart)

      const result = await commands.getContextLines(
        localDir,
        commitSha,
        filePath,
        fetchStart,
        fetchEnd,
        oldStartLine,
      )

      if (result.status === "error") {
        toast.error("Failed to expand context lines")
        return
      }

      setFetchedContextLines((prev) => {
        const next = new Map(prev)
        for (const line of result.data) {
          if (line.newLineno != null) {
            next.set(line.newLineno, line)
          }
        }
        return next
      })
    },
    [localDir, commitSha, filePath],
  )

  return { fetchedContextLines, handleExpandGap }
}
