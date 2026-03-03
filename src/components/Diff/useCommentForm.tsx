import { useState } from "react"

import { CommentContext, CommentLineState } from "./types"
import { UseLineSelectionReturn } from "./useLineSelection"

export function useCommentForm({
  selection,
  commentContext,
  filePath,
  commitSha,
}: {
  selection: UseLineSelectionReturn
  commentContext: CommentContext | undefined
  filePath: string
  commitSha: string
}) {
  const [commentLine, setCommentLine] = useState<CommentLineState>(null)

  const initiateComment = () => {
    if (!commentContext) return
    const resolved = selection.toCommentLineState()
    if (!resolved) return
    setCommentLine(resolved)
  }

  const handleSubmitComment = (body: string) => {
    if (!commentContext || !commentLine) return
    commentContext.onCreateComment({
      body,
      path: filePath,
      line: commentLine.line,
      side: commentLine.side,
      commitId: commitSha,
      startLine: commentLine.startLine,
      startSide: commentLine.startSide,
    })
    setCommentLine(null)
  }

  const cancelComment = () => {
    setCommentLine(null)
  }

  return {
    commentLine,
    isActive: commentLine != null,
    initiateComment,
    handleSubmitComment,
    cancelComment,
  }
}

export type UseCommentFormReturn = ReturnType<typeof useCommentForm>
