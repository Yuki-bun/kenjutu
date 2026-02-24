import { useEffect } from "react"

import { FileDiffItem, Header, useDiffContext } from "@/components/Diff"
import { InlineCommentForm } from "@/components/InlineCommentForm"
import { usePaneContext } from "@/components/Pane"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
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
}: {
  owner: string
  repo: string
  prNumber: number
}) {
  const { files, changeId } = useDiffContext()
  const createCommentMutation = useCreateReviewComment()
  useScrollCommentsOnFocus()

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
        {files.map((file) => (
          <FileDiffItem
            key={`${changeId}-${file.newPath || file.oldPath}`}
            file={file}
            commentContext={commentContext}
            InlineCommentForm={InlineCommentForm}
          />
        ))}
      </div>
    </div>
  )
}
