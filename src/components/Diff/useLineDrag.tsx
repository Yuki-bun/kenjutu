import { useCallback, useEffect, useRef, useState } from "react"

import {
  CommentLineState,
  InlineCommentFormProps,
  PRCommentContext,
} from "./types"

export function useLineDrag({
  filePath,
  commitSha,
  prComment,
  InlineCommentForm,
  onExitLineMode,
}: {
  filePath: string
  commitSha: string
  prComment: PRCommentContext | undefined
  InlineCommentForm: React.FC<InlineCommentFormProps> | undefined
  onExitLineMode: () => void
}) {
  const [commentLine, setCommentLine] = useState<CommentLineState>(null)
  const [isDragging, setIsDragging] = useState(false)
  const dragRef = useRef<{
    startLine: number
    side: "LEFT" | "RIGHT"
  } | null>(null)

  const handleLineDragStart = prComment
    ? (line: number, side: "LEFT" | "RIGHT") => {
        dragRef.current = { startLine: line, side }
        setIsDragging(true)
        setCommentLine({ line, side })
      }
    : undefined

  const handleLineDragEnter = prComment
    ? (line: number, side: "LEFT" | "RIGHT") => {
        if (!dragRef.current || dragRef.current.side !== side) return
        const startLine = Math.min(dragRef.current.startLine, line)
        const endLine = Math.max(dragRef.current.startLine, line)
        setCommentLine(
          startLine === endLine
            ? { line: endLine, side }
            : { line: endLine, side, startLine, startSide: side },
        )
      }
    : undefined

  const handleLineDragEnd = prComment
    ? () => {
        dragRef.current = null
        setIsDragging(false)
      }
    : undefined

  // End drag on mouseup anywhere (in case user releases outside gutter)
  useEffect(() => {
    const onMouseUp = () => {
      if (dragRef.current) {
        dragRef.current = null
        setIsDragging(false)
      }
    }
    document.addEventListener("mouseup", onMouseUp)
    return () => document.removeEventListener("mouseup", onMouseUp)
  }, [])

  const handleLineComment = useCallback(
    (comment: NonNullable<CommentLineState>) => {
      setCommentLine(comment)
      onExitLineMode()
    },
    [onExitLineMode],
  )

  const handleSubmitComment = (body: string) => {
    if (!prComment || !commentLine) return
    prComment.onCreateComment({
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

  const commentForm =
    InlineCommentForm && commentLine && !isDragging ? (
      <InlineCommentForm
        onSubmit={handleSubmitComment}
        onCancel={() => setCommentLine(null)}
      />
    ) : null

  return {
    commentLine,
    handleLineDragStart,
    handleLineDragEnter,
    handleLineDragEnd,
    handleLineComment,
    commentForm,
  }
}
