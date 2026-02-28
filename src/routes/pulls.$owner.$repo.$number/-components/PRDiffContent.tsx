import { useCallback, useEffect, useMemo } from "react"

import { FileDiffItem, Header, useDiffContext } from "@/components/Diff"
import { InlineCommentForm } from "@/components/InlineCommentForm"
import { usePaneContext } from "@/components/Pane"
import { useShaToChangeId } from "@/context/ShaToChangeIdContext"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import {
  buildCommentThreads,
  type ReviewComment,
  type ThreadedComment,
  useReviewComments,
} from "../-hooks/useReviewComments"
import { InlineCommentThread } from "./InlineCommentThread"
import { focusFileComment } from "./ReviewCommentsSidebar"

function useScrollCommentsOnFocus() {
  const { focusedId: filePath } = usePaneContext()

  useEffect(() => {
    if (filePath) {
      focusFileComment(filePath)
    }
  }, [filePath])

  return null
}

export function PRDiffContent({
  owner,
  repo,
  prNumber,
  remoteUrls,
}: {
  owner: string
  repo: string
  prNumber: number
  remoteUrls: string[]
}) {
  const { files, changeId, commitSha, localDir } = useDiffContext()
  const createCommentMutation = useCreateReviewComment()
  useScrollCommentsOnFocus()

  const { data: allComments } = useReviewComments(owner, repo, prNumber)
  const { getChangeId } = useShaToChangeId()

  // Filter comments for the current commit
  const commentsByFile = useMemo(() => {
    const map = new Map<string, ReviewComment[]>()
    if (!allComments) return map

    for (const comment of allComments) {
      const commentChangeId = getChangeId(
        comment.original_commit_id,
        localDir,
        remoteUrls,
      )
      const matches =
        commentChangeId != null
          ? commentChangeId === changeId
          : comment.original_commit_id === commitSha

      if (matches) {
        const existing = map.get(comment.path) ?? []
        existing.push(comment)
        map.set(comment.path, existing)
      }
    }

    return map
  }, [allComments, getChangeId, localDir, remoteUrls, changeId, commitSha])

  const inlineThreadNodes = useMemo(() => {
    const result = new Map<string, Map<string, React.ReactNode>>()

    for (const [filePath, fileComments] of commentsByFile) {
      const threads = buildCommentThreads(fileComments)
      if (threads.length === 0) continue

      const nodesByKey = new Map<string, React.ReactNode>()
      // Group threads by line:side
      const threadsByKey = new Map<string, ThreadedComment[]>()
      for (const thread of threads) {
        const line = thread.root.line ?? thread.root.original_line
        if (line == null) continue
        const key = `${line}:${thread.root.side}`
        const existing = threadsByKey.get(key) ?? []
        existing.push(thread)
        threadsByKey.set(key, existing)
      }

      for (const [key, keyThreads] of threadsByKey) {
        nodesByKey.set(
          key,
          <InlineThreadGroup
            threads={keyThreads}
            owner={owner}
            repo={repo}
            prNumber={prNumber}
          />,
        )
      }

      result.set(filePath, nodesByKey)
    }

    return result
  }, [commentsByFile, owner, repo, prNumber])

  const makeGetInlineThreads = useCallback(
    (filePath: string) => {
      const nodesByKey = inlineThreadNodes.get(filePath)
      if (!nodesByKey) return undefined

      return (line: number, side: "LEFT" | "RIGHT") =>
        nodesByKey.get(`${line}:${side}`) ?? null
    },
    [inlineThreadNodes],
  )

  const handleCreateComment = async (params: {
    body: string
    path: string
    line: number
    side: "LEFT" | "RIGHT"
    commitId: string
    startLine?: number
    startSide?: "LEFT" | "RIGHT"
  }) => {
    await createCommentMutation.mutateAsync({
      type: "new",
      owner,
      repo,
      pullNumber: prNumber,
      body: params.body,
      commitId: params.commitId,
      path: params.path,
      line: params.line,
      side: params.side,
      startLine: params.startLine,
      startSide: params.startSide,
    })
  }

  const commentContext = {
    onCreateComment: handleCreateComment,
  }

  return (
    <div className="space-y-2">
      <Header />
      <div className="space-y-3">
        {files.map((file) => {
          const filePath = file.newPath || file.oldPath || ""
          return (
            <FileDiffItem
              key={`${changeId}-${filePath}`}
              file={file}
              commentContext={commentContext}
              InlineCommentForm={InlineCommentForm}
              getInlineThreads={makeGetInlineThreads(filePath)}
            />
          )
        })}
      </div>
    </div>
  )
}

function InlineThreadGroup({
  threads,
  owner,
  repo,
  prNumber,
}: {
  threads: ThreadedComment[]
  owner: string
  repo: string
  prNumber: number
}) {
  return (
    <>
      {threads.map((thread) => (
        <InlineCommentThread
          key={thread.root.id}
          thread={thread}
          owner={owner}
          repo={repo}
          prNumber={prNumber}
        />
      ))}
    </>
  )
}
