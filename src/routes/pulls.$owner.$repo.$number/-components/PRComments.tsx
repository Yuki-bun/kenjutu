import { MarkdownContent } from "@/components/MarkdownContent"
import { formatRelativeTime } from "@/lib/timeUtils"

import {
  type GitHubIssueComment,
  usePullRequestComments,
} from "../-hooks/usePullRequestComments"
import {
  CommentCard,
  CommentCardContent,
  CommentCardHeader,
} from "./CommentCard"
import { UserAvatar } from "./UserAvatar"

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
      <UserAvatar user={comment.user} />
      <div className="flex-1 min-w-0">
        <CommentCard>
          <CommentCardHeader>
            <div className="flex items-baseline gap-2">
              <span className="text-sm font-semibold">
                {comment.user?.login}
              </span>
              <span className="text-xs text-muted-foreground">
                commented {formatRelativeTime(comment.created_at)}
              </span>
            </div>
          </CommentCardHeader>
          <CommentCardContent>
            <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
          </CommentCardContent>
        </CommentCard>
      </div>
    </div>
  )
}
