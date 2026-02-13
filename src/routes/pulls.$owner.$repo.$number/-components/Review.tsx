import { MarkdownContent } from "@/components/MarkdownContent"
import { formatRelativeTime } from "@/lib/timeUtils"
import { cn } from "@/lib/utils"

import { GitHubReview } from "../-hooks/usePullRequestReviews"
import { GithubReviewComment } from "../-hooks/useReviewComments"
import {
  CommentCard,
  CommentCardContent,
  CommentCardHeader,
} from "./CommentCard"
import { UserAvatar } from "./UserAvatar"

export type Review = {
  type: "review"
  review: GitHubReview
  comments: GithubReviewComment[]
}

export function Review({ review }: { review: Review }) {
  const reviewStateLabel =
    {
      APPROVED: "approved",
      CHANGES_REQUESTED: "requested changes",
      COMMENTED: "reviewed",
      DISMISSED: "dismissed",
      PENDING: "started a review",
    }[review.review.state] ?? "reviewed"

  return (
    <div className="flex gap-3">
      <UserAvatar user={review.review.user} />
      <div className="flex-1 min-w-0 space-y-3">
        <CommentCard>
          <CommentCardHeader>
            <div className="flex items-baseline gap-2">
              <span className="text-sm font-semibold">
                {review.review.user?.login}
              </span>
              {review.review.submitted_at && (
                <span className="text-xs text-muted-foreground">
                  {reviewStateLabel}{" "}
                  {formatRelativeTime(review.review.submitted_at)}
                </span>
              )}
            </div>
          </CommentCardHeader>
          <CommentCardContent className={cn(!review.review.body && "hidden")}>
            <MarkdownContent>{review.review.body}</MarkdownContent>
          </CommentCardContent>
        </CommentCard>
        {review.comments.length > 0 && (
          <div className="space-y-3 pl-4 border-l-2 border-muted">
            {review.comments.map((comment) => (
              <ReviewCommentItem key={comment.id} comment={comment} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function ReviewCommentItem({ comment }: { comment: GithubReviewComment }) {
  return (
    <CommentCard>
      <CommentCardHeader>
        <div className="flex items-baseline gap-2">
          <span className="text-xs font-semibold">{comment.user?.login}</span>
          <span className="text-xs text-muted-foreground">
            commented on {comment.path}
            {comment.line && `:${comment.line}`}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatRelativeTime(comment.created_at)}
          </span>
        </div>
      </CommentCardHeader>
      <CommentCardContent>
        <MarkdownContent>{comment.body ?? ""}</MarkdownContent>
      </CommentCardContent>
    </CommentCard>
  )
}
