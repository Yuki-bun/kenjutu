export type PRCommentContext = {
  onCreateComment: (params: {
    body: string
    path: string
    line: number
    side: "LEFT" | "RIGHT"
    commitId: string
    startLine?: number
    startSide?: "LEFT" | "RIGHT"
  }) => Promise<void>
}

export type InlineCommentFormProps = {
  onSubmit: (body: string) => void
  onCancel: () => void
}

export type CommentLineState = {
  line: number
  side: "LEFT" | "RIGHT"
  startLine?: number
  startSide?: "LEFT" | "RIGHT"
} | null
