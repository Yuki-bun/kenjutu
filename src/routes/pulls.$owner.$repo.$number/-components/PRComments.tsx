import { User } from "lucide-react"

import { MarkdownContent } from "@/components/MarkdownContent"
import { formatRelativeTime } from "@/lib/timeUtils"

import {
  type GitHubIssueComment,
  usePullRequestComments,
} from "../-hooks/usePullRequestComments"

type PRCommentsProps = {
  owner: string
  repo: string
  number: number
}

export function PRComments({ owner, repo, number }: PRCommentsProps) {
  const {
    data: comments,
    isLoading,
    error,
  } = usePullRequestComments(owner, repo, number)

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-medium">
        Comments ({comments?.length || 0})
      </h3>
      <div className="space-y-4">
        {isLoading && (
          <p className="text-sm text-muted-foreground">Loading comments...</p>
        )}
        {error && (
          <p className="text-sm text-destructive">
            Failed to load comments: {error.message}
          </p>
        )}
        {!isLoading && !error && comments?.length === 0 && (
          <p className="text-sm text-muted-foreground">No comments yet</p>
        )}
        {comments?.map((comment) => (
          <CommentItem key={comment.id} comment={comment} />
        ))}
      </div>
    </div>
  )
}

function CommentItem({ comment }: { comment: GitHubIssueComment }) {
  return (
    <div className="flex gap-3">
      <div className="w-10 h-10 rounded-full bg-muted flex items-center justify-center text-sm font-medium shrink-0 overflow-hidden">
        {comment.user?.avatar_url ? (
          <img
            src={comment.user.avatar_url}
            alt={comment.user.login}
            className="w-full h-full object-cover"
          />
        ) : (
          <User className="w-5 h-5 text-muted-foreground" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="rounded-lg border bg-card">
          <div className="px-4 py-3 border-b bg-muted/30">
            <div className="flex items-baseline gap-2">
              <span className="text-sm font-semibold">
                {comment.user?.login}
              </span>
              <span className="text-xs text-muted-foreground">
                commented {formatRelativeTime(comment.created_at)}
              </span>
            </div>
          </div>
          <div className="px-4 py-3">
            <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
          </div>
        </div>
      </div>
    </div>
  )
}
