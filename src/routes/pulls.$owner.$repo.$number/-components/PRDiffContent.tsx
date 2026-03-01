import { useEffect } from "react"

import { PRCommit } from "@/bindings"
import {
  type CommentContext,
  FileDiffItem,
  Header,
  useDiffContext,
} from "@/components/Diff"
import { InlineCommentForm } from "@/components/InlineCommentForm"
import { usePaneContext } from "@/components/Pane"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import { useNormalizedReviewComments } from "../-hooks/useNormalizedReviewComments"
import { useReviewComments } from "../-hooks/useReviewComments"
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
  currentCommit,
  localDir,
  remoteUrls,
}: {
  owner: string
  repo: string
  prNumber: number
  currentCommit: PRCommit
  localDir: string
  remoteUrls: string[]
}) {
  const { files, changeId } = useDiffContext()
  const createCommentMutation = useCreateReviewComment()
  useScrollCommentsOnFocus()

  const { data: reviewComments } = useReviewComments(owner, repo, prNumber)
  const normalizedComments = useNormalizedReviewComments(
    reviewComments,
    currentCommit,
    localDir,
    remoteUrls,
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

  const handleReplyToThread = async (threadId: string, body: string) => {
    // threadId is the stringified root comment id
    const rootId = Number(threadId)
    // Find the root comment to get path + commitId for the reply
    const rootComment = reviewComments?.find((c) => c.id === rootId)
    if (!rootComment) return

    await createCommentMutation.mutateAsync({
      type: "reply",
      owner,
      repo,
      pullNumber: prNumber,
      body,
      inReplyTo: rootId,
      commitId: rootComment.original_commit_id,
      path: rootComment.path,
    })
  }

  const commentContext: CommentContext = {
    onCreateComment: handleCreateComment,
    onReplyToThread: handleReplyToThread,
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
              inlineComments={normalizedComments.get(filePath)}
            />
          )
        })}
      </div>
    </div>
  )
}
