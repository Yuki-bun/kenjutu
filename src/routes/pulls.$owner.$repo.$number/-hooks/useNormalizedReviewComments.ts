import { useMemo } from "react"

import { PRCommit } from "@/bindings"
import {
  inlineCommentsKey,
  InlineCommentsMap,
  InlineThread,
} from "@/components/Diff/"
import { useShaToChangeId } from "@/context/ShaToChangeIdContext"

import { ReviewComment } from "./useReviewComments"

function toInlineThread(
  root: ReviewComment,
  replies: ReviewComment[],
): InlineThread {
  const line = root.line ?? root.original_line
  if (line == null) return null as never // file-level comments are skipped

  return {
    id: String(root.id),
    body: root.body,
    createdAt: root.created_at,
    user: root.user
      ? { login: root.user.login, avatarUrl: root.user.avatar_url }
      : undefined,
    replies: replies.map((r) => ({
      id: String(r.id),
      body: r.body,
      createdAt: r.created_at,
      user: r.user
        ? { login: r.user.login, avatarUrl: r.user.avatar_url }
        : undefined,
    })),
    line,
    startLine: root.start_line ?? root.original_start_line ?? undefined,
    side: root.side,
  }
}

/**
 * Normalizes PR review comments into per-file `InlineCommentsMap` for
 * rendering inline in the diff viewer.
 */
export function useNormalizedReviewComments(
  comments: ReviewComment[] | undefined,
  currentCommit: PRCommit,
  localDir: string | null,
  remoteUrls: string[],
): Map<string, InlineCommentsMap> {
  const { getChangeId } = useShaToChangeId()

  return useMemo(() => {
    const result = new Map<string, InlineCommentsMap>()
    if (!comments) return result

    // Filter to current commit using changeId matching (same as sidebar)
    const filtered = comments.filter((comment) => {
      const commentChangeId = getChangeId(
        comment.original_commit_id,
        localDir,
        remoteUrls,
      )
      if (commentChangeId == null) {
        return comment.original_commit_id === currentCommit.sha
      }
      return commentChangeId === currentCommit.changeId
    })

    // Group by path
    const byPath = new Map<string, ReviewComment[]>()
    for (const c of filtered) {
      const list = byPath.get(c.path) ?? []
      list.push(c)
      byPath.set(c.path, list)
    }

    for (const [filePath, fileComments] of byPath) {
      const roots = fileComments.filter((c) => !c.in_reply_to_id)
      const replies = fileComments.filter((c) => c.in_reply_to_id)

      const lineMap: InlineCommentsMap = new Map()

      for (const root of roots) {
        // Skip file-level comments (no line number)
        const line = root.line ?? root.original_line
        if (line == null) continue

        const threadReplies = replies
          .filter((r) => r.in_reply_to_id === root.id)
          .sort(
            (a, b) =>
              new Date(a.created_at).getTime() -
              new Date(b.created_at).getTime(),
          )

        const thread = toInlineThread(root, threadReplies)
        const key = inlineCommentsKey(root.side, line)
        const existing = lineMap.get(key) ?? []
        existing.push(thread)
        lineMap.set(key, existing)
      }

      if (lineMap.size > 0) {
        result.set(filePath, lineMap)
      }
    }

    return result
  }, [comments, currentCommit, localDir, remoteUrls, getChangeId])
}
