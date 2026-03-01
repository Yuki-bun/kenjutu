export type CommentContext = {
  onCreateComment: (params: {
    body: string
    path: string
    line: number
    side: "LEFT" | "RIGHT"
    commitId: string
    startLine?: number
    startSide?: "LEFT" | "RIGHT"
  }) => Promise<void>
  onReplyToThread?: (threadId: string, body: string) => Promise<void>
}

export type InlineCommentFormProps = {
  onSubmit: (body: string) => void
  onCancel: () => void
  placeholder?: string
}

export type CommentLineState = {
  line: number
  side: "LEFT" | "RIGHT"
  startLine?: number
  startSide?: "LEFT" | "RIGHT"
} | null

// -- Inline comment display types --

export type InlineCommentUser = {
  login: string
  avatarUrl: string
}

export type InlineReply = {
  id: string
  body: string
  createdAt: string
  user?: InlineCommentUser
}

export type InlineThread = {
  id: string
  body: string
  createdAt: string
  user?: InlineCommentUser
  replies: InlineReply[]
  line: number
  startLine?: number
  side: "LEFT" | "RIGHT"
  resolved?: boolean
  isPorted?: boolean
}

export type CommentKey = string & { __brand: "CommentKey" }

export type InlineCommentsMap = Map<CommentKey, InlineThread[]>

export function inlineCommentsKey(
  side: "LEFT" | "RIGHT",
  line: number,
): CommentKey {
  return `${side}:${line}` as CommentKey
}
