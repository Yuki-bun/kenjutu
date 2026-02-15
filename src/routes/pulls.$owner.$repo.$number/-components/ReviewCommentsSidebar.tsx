import { ChevronDown, ChevronRight } from "lucide-react"
import { useState } from "react"

import { MarkdownContent } from "@/components/MarkdownContent"
import { Badge } from "@/components/ui/badge"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { useShaToChangeId } from "@/context/ShaToChangeIdContext"
import { compareFilePaths } from "@/lib/fileTree"
import { formatRelativeTime } from "@/lib/timeUtils"

import { type PRCommit } from "../-hooks/usePullRequest"
import { type GithubReviewComment } from "../-hooks/useReviewComments"
import { CommentCard } from "./CommentCard"

type ReviewCommentsSidebarProps = {
  comments: GithubReviewComment[]
  currentCommit: PRCommit
  localDir: string | null
}

type ThreadedComment = {
  root: GithubReviewComment
  replies: GithubReviewComment[]
  lineNumber: number
}

type ThreadedFileComments = {
  filePath: string
  threads: ThreadedComment[]
  orphanedReplies: GithubReviewComment[]
}

export function ReviewCommentsSidebar({
  comments,
  currentCommit,
  localDir,
}: ReviewCommentsSidebarProps) {
  const { getChangeId } = useShaToChangeId()

  const commitsForCurrentCommit = comments.filter((comment) => {
    const commentChangeId = getChangeId(comment.original_commit_id, localDir)
    if (commentChangeId == null || currentCommit.changeId == null) {
      return comment.original_commit_id === currentCommit.sha
    }
    return commentChangeId === currentCommit.changeId
  })

  const commentsByPath = commitsForCurrentCommit.reduce<
    Map<string, GithubReviewComment[]>
  >((acc, comment) => {
    const path = comment.path
    const existing = acc.get(path) ?? []
    acc.set(path, [...existing, comment])
    return acc
  }, new Map())

  const fileComments = Array.from(commentsByPath.entries())
    .map(([filePath, comments]) => {
      // Separate root comments from replies
      const rootComments = comments.filter((c) => !c.in_reply_to_id)
      const replyComments = comments.filter((c) => c.in_reply_to_id)

      // Create thread structure
      const threads: ThreadedComment[] = rootComments.map((root) => {
        const replies = replyComments
          .filter((reply) => reply.in_reply_to_id === root.id)
          .sort(
            (a, b) =>
              new Date(a.created_at).getTime() -
              new Date(b.created_at).getTime(),
          )

        const lineNumber = root.line ?? root.original_line ?? 0

        return {
          root,
          replies,
          lineNumber,
        }
      })

      // Sort threads by line number
      threads.sort((a, b) => a.lineNumber - b.lineNumber)

      // Find orphaned replies (parent not in current commit filter)
      const allRepliesInThreads = new Set(
        threads.flatMap((t) => t.replies.map((r) => r.id)),
      )
      const orphanedReplies = replyComments.filter(
        (reply) => !allRepliesInThreads.has(reply.id),
      )

      return {
        filePath,
        threads,
        orphanedReplies,
      }
    })
    .sort(compareFilePaths((file) => file.filePath))

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

function FileCommentsSection({
  fileComments,
}: {
  fileComments: ThreadedFileComments
}) {
  const [isOpen, setIsOpen] = useState(true)

  const totalCount =
    fileComments.threads.reduce(
      (sum, thread) => sum + 1 + thread.replies.length,
      0,
    ) + fileComments.orphanedReplies.length

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger className="flex items-center gap-2 w-full text-left hover:bg-muted/50 p-2 rounded transition-colors">
        {isOpen ? (
          <ChevronDown className="w-4 h-4 shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 shrink-0" />
        )}
        <span className="text-xs font-medium truncate flex-1">
          {fileComments.filePath}
        </span>
        <Badge variant="secondary" className="shrink-0">
          {totalCount}
        </Badge>
      </CollapsibleTrigger>

      <CollapsibleContent className="mt-2 ml-6 space-y-2">
        {fileComments.threads.map((thread) => (
          <CommentThread key={thread.root.id} thread={thread} />
        ))}
        {fileComments.orphanedReplies.map((reply) => (
          <OrphanedReplyComment key={reply.id} comment={reply} />
        ))}
      </CollapsibleContent>
    </Collapsible>
  )
}

function CommentThread({ thread }: { thread: ThreadedComment }) {
  const isDeletedLine = !thread.root.line && thread.root.original_line
  const displayLine = thread.root.line ?? thread.root.original_line

  return (
    <CommentCard>
      {/* Root Comment */}
      <div className="p-4">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0 overflow-hidden">
            {thread.root.user?.avatar_url ? (
              <img
                src={thread.root.user.avatar_url}
                alt={thread.root.user.login}
                className="w-full h-full object-cover"
              />
            ) : (
              thread.root.user?.login?.[0]?.toUpperCase() || "?"
            )}
          </div>
          <span className="font-semibold text-sm">
            {thread.root.user?.login}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatRelativeTime(thread.root.created_at)}
          </span>
          <div className="ml-auto flex items-center gap-2">
            {displayLine && (
              <span
                className={
                  isDeletedLine
                    ? "text-xs text-destructive"
                    : "text-xs text-muted-foreground"
                }
              >
                Line {displayLine}
                {isDeletedLine && " (deleted)"}
              </span>
            )}
          </div>
        </div>
        <MarkdownContent>{thread.root.body ?? ""}</MarkdownContent>
      </div>

      {/* Replies */}
      {thread.replies.length > 0 &&
        thread.replies.map((reply) => (
          <div key={reply.id} className="border-t p-4">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0 overflow-hidden">
                {reply.user?.avatar_url ? (
                  <img
                    src={reply.user.avatar_url}
                    alt={reply.user.login}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  reply.user?.login?.[0]?.toUpperCase() || "?"
                )}
              </div>
              <span className="font-semibold text-sm">{reply.user?.login}</span>
              <span className="text-xs text-muted-foreground">
                {formatRelativeTime(reply.created_at)}
              </span>
            </div>
            <MarkdownContent>{reply.body ?? ""}</MarkdownContent>
          </div>
        ))}
    </CommentCard>
  )
}

function OrphanedReplyComment({ comment }: { comment: GithubReviewComment }) {
  return (
    <CommentCard className="border-dashed border-muted-foreground/50 opacity-90">
      <div className="p-4">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0 overflow-hidden">
            {comment.user?.avatar_url ? (
              <img
                src={comment.user.avatar_url}
                alt={comment.user.login}
                className="w-full h-full object-cover"
              />
            ) : (
              comment.user?.login?.[0]?.toUpperCase() || "?"
            )}
          </div>
          <span className="font-semibold text-sm">{comment.user?.login}</span>
          <span className="text-xs text-muted-foreground">
            {formatRelativeTime(comment.created_at)}
          </span>
          <span className="text-xs text-muted-foreground italic ml-2">
            Reply to comment on different commit
          </span>
        </div>
        <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
      </div>
    </CommentCard>
  )
}
