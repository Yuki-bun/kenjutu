import * as Collapsible from "@radix-ui/react-collapsible"
import { ChevronDown, ChevronRight } from "lucide-react"
import { useMemo, useState } from "react"

import { MarkdownContent } from "@/components/MarkdownContent"
import { Badge } from "@/components/ui/badge"
import { useShaToChangeId } from "@/context/ShaToChangeIdContext"
import { formatRelativeTime } from "@/lib/timeUtils"

import {
  CommentCard,
  CommentCardContent,
  CommentCardHeader,
} from "../../routes/pulls.$owner.$repo.$number/-components/CommentCard"
import { type PRCommit } from "../../routes/pulls.$owner.$repo.$number/-hooks/usePullRequest"
import { type GithubReviewComment } from "../../routes/pulls.$owner.$repo.$number/-hooks/useReviewComments"

type ReviewCommentsSidebarProps = {
  comments: GithubReviewComment[]
  currentCommit: PRCommit | undefined
}

type FileComments = {
  filePath: string
  comments: GithubReviewComment[]
}

export function ReviewCommentsSidebar({
  comments,
  currentCommit,
}: ReviewCommentsSidebarProps) {
  const { getChangeId } = useShaToChangeId()

  const commitsForCurrentCommit = useMemo(() => {
    if (!currentCommit) return []

    return comments.filter((comment) => {
      const commentChangeId = getChangeId(comment.original_commit_id)
      if (commentChangeId === undefined) return false
      if (commentChangeId === null) {
        return comment.original_commit_id === currentCommit.sha
      }
      return currentCommit.changeId
        ? commentChangeId === currentCommit.changeId
        : comment.original_commit_id === currentCommit.sha
    })
  }, [comments, currentCommit, getChangeId])

  // Group comments by file
  const fileComments = useMemo<FileComments[]>(() => {
    const grouped = new Map<string, GithubReviewComment[]>()

    commitsForCurrentCommit.forEach((comment) => {
      const existing = grouped.get(comment.path) ?? []
      grouped.set(comment.path, [...existing, comment])
    })

    return Array.from(grouped.entries())
      .map(([filePath, comments]) => ({
        filePath,
        comments: comments.sort((a, b) => {
          const lineA = a.line ?? a.original_line ?? 0
          const lineB = b.line ?? b.original_line ?? 0
          return lineA - lineB
        }),
      }))
      .sort((a, b) => a.filePath.localeCompare(b.filePath))
  }, [commitsForCurrentCommit])

  const totalComments = commitsForCurrentCommit.length

  if (!currentCommit) {
    return (
      <div className="p-4">
        <h2 className="text-sm font-semibold mb-2">Review Comments</h2>
        <p className="text-xs text-muted-foreground">
          Select a commit to view review comments
        </p>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Review Comments</h2>
          {totalComments > 0 && (
            <Badge variant="secondary">{totalComments}</Badge>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {totalComments === 0 ? (
          <div className="p-4">
            <p className="text-xs text-muted-foreground">
              No review comments for this commit
            </p>
          </div>
        ) : (
          <div className="p-4 space-y-3">
            {fileComments.map((fileComment) => (
              <FileCommentsSection
                key={fileComment.filePath}
                fileComments={fileComment}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function FileCommentsSection({ fileComments }: { fileComments: FileComments }) {
  const [isOpen, setIsOpen] = useState(true)

  return (
    <Collapsible.Root open={isOpen} onOpenChange={setIsOpen}>
      <Collapsible.Trigger className="flex items-center gap-2 w-full text-left hover:bg-muted/50 p-2 rounded transition-colors">
        {isOpen ? (
          <ChevronDown className="w-4 h-4 shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 shrink-0" />
        )}
        <span className="text-xs font-medium truncate flex-1">
          {fileComments.filePath}
        </span>
        <Badge variant="secondary" className="shrink-0">
          {fileComments.comments.length}
        </Badge>
      </Collapsible.Trigger>

      <Collapsible.Content className="mt-2 ml-6 space-y-2">
        {fileComments.comments.map((comment) => (
          <CommentCard key={comment.id}>
            <CommentCardHeader>
              <div className="flex items-baseline gap-2 flex-wrap">
                <span className="text-xs font-semibold">
                  {comment.user?.login}
                </span>
                {comment.line && (
                  <span className="text-xs text-muted-foreground">
                    Line {comment.line}
                  </span>
                )}
                <span className="text-xs text-muted-foreground">
                  {formatRelativeTime(comment.created_at)}
                </span>
              </div>
            </CommentCardHeader>
            <CommentCardContent>
              <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
            </CommentCardContent>
          </CommentCard>
        ))}
      </Collapsible.Content>
    </Collapsible.Root>
  )
}
