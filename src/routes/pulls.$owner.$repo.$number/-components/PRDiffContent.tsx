import { GripVertical } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import { type PRCommit } from "@/bindings"
import { FileDiffItem, Header, useDiffContext } from "@/components/Diff"
import { MarkdownContent } from "@/components/MarkdownContent"
import { useShaToChangeId } from "@/context/ShaToChangeIdContext"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import { useReviewComments } from "../-hooks/useReviewComments"
import { InlineCommentForm } from "./InlineCommentForm"
import {
  FileReviewComments,
  filterCommentsForCommit,
  threadCommentsForFile,
} from "./ReviewCommentsSidebar"

const MIN_COMMENT_WIDTH = 200
const MAX_COMMENT_WIDTH = 600
const DEFAULT_COMMENT_WIDTH = 350

function useColumnResize(defaultWidth: number) {
  const [width, setWidth] = useState(defaultWidth)
  const [isDragging, setIsDragging] = useState(false)
  const dragState = useRef<{ startX: number; startWidth: number } | null>(null)

  useEffect(() => {
    if (!isDragging) return

    const onMouseMove = (e: MouseEvent) => {
      if (!dragState.current) return
      const delta = dragState.current.startX - e.clientX
      const newWidth = Math.min(
        MAX_COMMENT_WIDTH,
        Math.max(MIN_COMMENT_WIDTH, dragState.current.startWidth + delta),
      )
      setWidth(newWidth)
    }

    const onMouseUp = () => {
      dragState.current = null
      setIsDragging(false)
    }

    document.body.style.cursor = "col-resize"
    document.body.style.userSelect = "none"
    document.addEventListener("mousemove", onMouseMove)
    document.addEventListener("mouseup", onMouseUp)
    return () => {
      document.body.style.cursor = ""
      document.body.style.userSelect = ""
      document.removeEventListener("mousemove", onMouseMove)
      document.removeEventListener("mouseup", onMouseUp)
    }
  }, [isDragging])

  const onDragStart = (e: React.MouseEvent) => {
    e.preventDefault()
    dragState.current = { startX: e.clientX, startWidth: width }
    setIsDragging(true)
  }

  return { width, onDragStart }
}

export function PRDiffContent({
  owner,
  repo,
  prNumber,
  currentCommit,
  localDir,
}: {
  owner: string
  repo: string
  prNumber: number
  currentCommit: PRCommit
  localDir: string | null
}) {
  const { files, changeId } = useDiffContext()
  const createCommentMutation = useCreateReviewComment()
  const { data: comments } = useReviewComments(owner, repo, prNumber)
  const { getChangeId } = useShaToChangeId()
  const { width: commentWidth, onDragStart } = useColumnResize(
    DEFAULT_COMMENT_WIDTH,
  )

  const commentsForCommit = filterCommentsForCommit(
    comments ?? [],
    currentCommit,
    getChangeId,
    localDir,
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

  const prComment = {
    onCreateComment: handleCreateComment,
    isCommentPending: createCommentMutation.isPending,
  }

  return (
    <div
      className="grid"
      style={{
        gridTemplateColumns: `1fr ${commentWidth}px`,
        columnGap: `17px`,
      }}
    >
      {/* Separator — absolutely positioned, full height */}
      <div
        className="absolute top-0 bottom-0 flex w-px items-start justify-center bg-border cursor-col-resize"
        style={{ left: `calc(100% - ${commentWidth}px - 8px)` }}
        onMouseDown={onDragStart}
      >
        <div className="sticky top-1/2 z-10 flex h-4 w-3 -translate-y-1/2 items-center justify-center rounded-sm border bg-border">
          <GripVertical className="h-2.5 w-2.5" />
        </div>
      </div>

      {/* Commit detail — left column only */}
      <div className="col-start-1 mb-4">
        <CommitDetail commit={currentCommit} />
      </div>

      {/* Changes header — left column only */}
      <div className="col-start-1 mb-2">
        <Header />
      </div>

      {/* File rows */}
      {files.map((file) => {
        const filePath = file.newPath || file.oldPath || ""
        const fileComments = threadCommentsForFile(commentsForCommit, filePath)

        return (
          <div
            key={`${changeId}-${filePath}`}
            className="grid grid-cols-subgrid col-span-2 mb-3"
          >
            <div className="min-w-0">
              <FileDiffItem
                file={file}
                prComment={prComment}
                InlineCommentForm={InlineCommentForm}
              />
            </div>
            <div className="pt-10">
              <FileReviewComments
                fileComments={fileComments}
                owner={owner}
                repo={repo}
                prNumber={prNumber}
              />
            </div>
          </div>
        )
      })}
    </div>
  )
}

function CommitDetail({ commit }: { commit: PRCommit }) {
  return (
    <div className="p-4 border rounded">
      <h3 className="font-semibold mb-2">
        {commit.summary || "(no description)"}
      </h3>
      {commit.description && (
        <MarkdownContent>{commit.description}</MarkdownContent>
      )}
      <div className="text-sm text-muted-foreground space-y-1 mt-1">
        <p>
          <span className="font-medium">Commit:</span>{" "}
          <code className="bg-muted px-1 rounded">
            {commit.sha.slice(0, 12)}
          </code>
        </p>
        <p>
          <span className="font-medium">Change ID:</span>{" "}
          <code className="bg-muted px-1 rounded">{commit.changeId}</code>
        </p>
      </div>
    </div>
  )
}
