import { Reply } from "lucide-react"
import { useState } from "react"

import { type PRCommit } from "@/bindings"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/lib/timeUtils"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import { type ReviewComment } from "../-hooks/useReviewComments"
import { CommentCard } from "./CommentCard"
import { InlineCommentForm } from "./InlineCommentForm"

export type ThreadedComment = {
  root: ReviewComment
  replies: ReviewComment[]
  lineNumber: number
}

export type ThreadedFileComments = {
  filePath: string
  threads: ThreadedComment[]
  orphanedReplies: ReviewComment[]
}

export function threadCommentsForFile(
  comments: ReviewComment[],
  filePath: string,
): ThreadedFileComments {
  const fileComments = comments.filter((c) => c.path === filePath)

  const rootComments = fileComments.filter((c) => !c.in_reply_to_id)
  const replyComments = fileComments.filter((c) => c.in_reply_to_id)

  const threads: ThreadedComment[] = rootComments.map((root) => {
    const replies = replyComments
      .filter((reply) => reply.in_reply_to_id === root.id)
      .sort(
        (a, b) =>
          new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      )

    const lineNumber = root.line ?? root.original_line ?? 0

    return { root, replies, lineNumber }
  })

  threads.sort((a, b) => a.lineNumber - b.lineNumber)

  const allRepliesInThreads = new Set(
    threads.flatMap((t) => t.replies.map((r) => r.id)),
  )
  const orphanedReplies = replyComments.filter(
    (reply) => !allRepliesInThreads.has(reply.id),
  )

  return { filePath, threads, orphanedReplies }
}

export function filterCommentsForCommit(
  comments: ReviewComment[],
  currentCommit: PRCommit,
  getChangeId: (
    sha: string,
    localDir: string | null,
  ) => string | null | undefined,
  localDir: string | null,
): ReviewComment[] {
  return comments.filter((comment) => {
    const commentChangeId = getChangeId(comment.original_commit_id, localDir)
    if (commentChangeId == null || currentCommit.changeId == null) {
      return comment.original_commit_id === currentCommit.sha
    }
    return commentChangeId === currentCommit.changeId
  })
}

export function FileReviewComments({
  fileComments,
  owner,
  repo,
  prNumber,
}: {
  fileComments: ThreadedFileComments
  owner: string
  repo: string
  prNumber: number
}) {
  if (
    fileComments.threads.length === 0 &&
    fileComments.orphanedReplies.length === 0
  ) {
    return null
  }

  return (
    <div className="space-y-2">
      {fileComments.threads.map((thread) => (
        <CommentThread
          key={thread.root.id}
          thread={thread}
          owner={owner}
          repo={repo}
          prNumber={prNumber}
        />
      ))}
      {fileComments.orphanedReplies.map((reply) => (
        <OrphanedReplyComment key={reply.id} comment={reply} />
      ))}
    </div>
  )
}

export function CommentThread({
  thread,
  owner,
  repo,
  prNumber,
}: {
  thread: ThreadedComment
  owner: string
  repo: string
  prNumber: number
}) {
  const [isReplying, setIsReplying] = useState(false)
  const createCommentMutation = useCreateReviewComment()

  const isDeletedLine = !thread.root.line && thread.root.original_line
  const displayLine = thread.root.line ?? thread.root.original_line
  const displayStartLine =
    thread.root.start_line ?? thread.root.original_start_line

  const handleReply = (body: string) => {
    createCommentMutation.mutateAsync({
      type: "reply",
      owner,
      repo,
      pullNumber: prNumber,
      body,
      inReplyTo: thread.root.id,
      commitId: thread.root.original_commit_id,
      path: thread.root.path,
    })
    setIsReplying(false)
  }

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
                {displayStartLine && displayStartLine !== displayLine
                  ? `Lines ${displayStartLine}-${displayLine}`
                  : `Line ${displayLine}`}
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

      {/* Reply section */}
      {isReplying ? (
        <div className="border-t">
          <InlineCommentForm
            onSubmit={handleReply}
            onCancel={() => setIsReplying(false)}
          />
        </div>
      ) : (
        <div className="border-t p-2">
          <Button
            variant="ghost"
            size="xs"
            onClick={() => setIsReplying(true)}
            className="w-full text-muted-foreground"
          >
            <Reply className="w-3 h-3" />
            Reply
          </Button>
        </div>
      )}
    </CommentCard>
  )
}

export function OrphanedReplyComment({ comment }: { comment: ReviewComment }) {
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
