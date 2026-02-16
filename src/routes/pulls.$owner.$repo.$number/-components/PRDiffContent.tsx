import { FileDiffItem, Header, useDiffContext } from "@/components/Diff"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import { InlineCommentForm } from "./InlineCommentForm"

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

  const prComment = {
    onCreateComment: handleCreateComment,
    isCommentPending: createCommentMutation.isPending,
  }

  return (
    <div className="space-y-2">
      <Header />
      <div className="space-y-3">
        {files.map((file) => (
          <FileDiffItem
            key={`${changeId}-${file.newPath || file.oldPath}`}
            file={file}
            prComment={prComment}
            InlineCommentForm={InlineCommentForm}
          />
        ))}
      </div>
    </div>
  )
}
