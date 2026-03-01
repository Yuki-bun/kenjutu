import { Reply } from "lucide-react"
import { useState } from "react"

import { InlineCommentForm } from "@/components/InlineCommentForm"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/lib/timeUtils"

import { useCreateReviewComment } from "../-hooks/useCreateReviewComment"
import {
  type ReviewComment,
  type ThreadedComment,
} from "../-hooks/useReviewComments"

function CommentAvatar({ user }: { user: ReviewComment["user"] }) {
  return (
    <div className="w-5 h-5 rounded-full bg-muted flex items-center justify-center text-[10px] font-medium shrink-0 overflow-hidden">
      {user?.avatar_url ? (
        <img
          src={user.avatar_url}
          alt={user.login}
          className="w-full h-full object-cover"
        />
      ) : (
        user?.login[0].toUpperCase() || "?"
      )}
    </div>
  )
}

function CommentBody({ comment }: { comment: ReviewComment }) {
  return (
    <div>
      <div className="flex items-center gap-1.5 mb-1">
        <CommentAvatar user={comment.user} />
        <span className="font-semibold text-xs">{comment.user?.login}</span>
        <span className="text-[10px] text-muted-foreground">
          {formatRelativeTime(comment.created_at)}
        </span>
      </div>
      <div className="pl-6.5 text-xs">
        <MarkdownContent>{comment.body}</MarkdownContent>
      </div>
    </div>
  )
}

export function InlineCommentThread({
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
    <div className="rounded border bg-card mx-2 my-1">
      {/* Root comment */}
      <div className="p-3">
        <CommentBody comment={thread.root} />
      </div>

      {/* Replies */}
      {thread.replies.map((reply) => (
        <div key={reply.id} className="border-t p-3">
          <CommentBody comment={reply} />
        </div>
      ))}

      {/* Reply section */}
      {isReplying ? (
        <div className="border-t">
          <InlineCommentForm
            onSubmit={handleReply}
            onCancel={() => setIsReplying(false)}
            placeholder="Write a reply..."
          />
        </div>
      ) : (
        <div className="border-t px-2 py-1">
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
    </div>
  )
}
