import { MarkdownContent } from "@/components/MarkdownContent"
import { formatRelativeTime } from "@/lib/timeUtils"
import { cn } from "@/lib/utils"

import {
  CommentCard,
  CommentCardContent,
  CommentCardHeader,
} from "../../routes/pulls.$owner.$repo.$number/-components/CommentCard"
import { GithubReviewComment } from "../../routes/pulls.$owner.$repo.$number/-hooks/useReviewComments"

type ReviewCommentThreadProps = {
  comments: GithubReviewComment[]
  lineNumber: number
}

/**
 * Displays a thread of review comments at a specific line in the diff.
 * Comments are grouped by line number and rendered with avatars, timestamps, and markdown.
 */
export function ReviewCommentThread({
  comments,
  lineNumber,
}: ReviewCommentThreadProps) {
  if (comments.length === 0) return null

  return (
    <div className="border-l-4 border-blue-500 bg-blue-50/50 dark:bg-blue-950/20 p-3 my-2">
      <div className="text-xs text-muted-foreground mb-2 font-semibold">
        Comments on line {lineNumber}
      </div>
      <div className="space-y-2">
        {comments.map((comment) => (
          <ReviewCommentCard key={comment.id} comment={comment} />
        ))}
      </div>
    </div>
  )
}

type ReviewCommentCardProps = {
  comment: GithubReviewComment
}

function ReviewCommentCard({ comment }: ReviewCommentCardProps) {
  return (
    <CommentCard className="bg-background">
      <CommentCardHeader className="py-1.5">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center overflow-hidden shrink-0">
            {comment.user?.avatar_url && (
              <img
                src={comment.user.avatar_url}
                alt={comment.user.login}
                className="w-full h-full object-cover"
              />
            )}
          </div>
          <span className="text-xs font-semibold">{comment.user?.login}</span>
          <span className="text-xs text-muted-foreground">
            {formatRelativeTime(comment.created_at)}
          </span>
          {comment.author_association &&
            comment.author_association !== "NONE" && (
              <ReviewStateBadge association={comment.author_association} />
            )}
        </div>
      </CommentCardHeader>
      <CommentCardContent className="py-2">
        <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
      </CommentCardContent>
    </CommentCard>
  )
}

type ReviewStateBadgeProps = {
  association: string
}

function ReviewStateBadge({ association }: ReviewStateBadgeProps) {
  const badgeConfig = {
    OWNER: {
      label: "Owner",
      className:
        "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
    },
    MEMBER: {
      label: "Member",
      className:
        "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
    },
    COLLABORATOR: {
      label: "Collaborator",
      className:
        "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
    },
    CONTRIBUTOR: {
      label: "Contributor",
      className:
        "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
    },
  }[association]

  if (!badgeConfig) return null

  return (
    <span
      className={cn(
        "text-xs px-1.5 py-0.5 rounded font-medium",
        badgeConfig.className,
      )}
    >
      {badgeConfig.label}
    </span>
  )
}
