import { useMemo } from "react"

import { DiffSide, FileComments } from "@/bindings"
import {
  inlineCommentsKey,
  InlineCommentsMap,
  InlineThread,
} from "@/components/Diff/"

function mapSide(side: DiffSide): "LEFT" | "RIGHT" {
  return side === "Old" ? "LEFT" : "RIGHT"
}

/**
 * Normalizes local ported comments into per-file `InlineCommentsMap` for
 * rendering inline in the diff viewer.
 */
export function useNormalizedLocalComments(
  fileComments: FileComments[] | undefined,
): Map<string, InlineCommentsMap> {
  return useMemo(() => {
    const result = new Map<string, InlineCommentsMap>()
    if (!fileComments) return result

    for (const fc of fileComments) {
      const lineMap: InlineCommentsMap = new Map()

      for (const ported of fc.comments) {
        const { comment } = ported
        const line = ported.ported_line ?? comment.line
        const side = mapSide(comment.side)

        const thread: InlineThread = {
          id: comment.id,
          body: comment.body,
          createdAt: comment.created_at,
          replies: comment.replies.map((r) => ({
            id: r.id,
            body: r.body,
            createdAt: r.created_at,
          })),
          line,
          startLine:
            ported.ported_start_line ?? comment.start_line ?? undefined,
          side,
          resolved: comment.resolved,
          isPorted: ported.is_ported,
        }

        const key = inlineCommentsKey(side, line)
        const existing = lineMap.get(key) ?? []
        existing.push(thread)
        lineMap.set(key, existing)
      }

      if (lineMap.size > 0) {
        result.set(fc.file_path, lineMap)
      }
    }

    return result
  }, [fileComments])
}
