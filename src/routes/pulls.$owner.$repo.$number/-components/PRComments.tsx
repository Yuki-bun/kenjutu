import { MarkdownContent } from "@/components/MarkdownContent"
import { formatRelativeTime } from "@/lib/timeUtils"

import {
  type GitHubIssueComment,
  usePullRequestComments,
} from "../-hooks/usePullRequestComments"
import { usePullRequestReviews } from "../-hooks/usePullRequestReviews"
import { useReviewComments } from "../-hooks/useReviewComments"
import {
  CommentCard,
  CommentCardContent,
  CommentCardHeader,
} from "./CommentCard"
import { Review } from "./Review"
import { UserAvatar } from "./UserAvatar"

type PRCommentsProps = {
  owner: string
  repo: string
  number: number
}

type Comment = GitHubIssueComment & {
  type: "comment"
}

export function PRComments({ owner, repo, number }: PRCommentsProps) {
  const {
    data: comments,
    isLoading,
    error,
  } = usePullRequestComments(owner, repo, number)

  const { data: reviews } = usePullRequestReviews(owner, repo, number)
  const { data: reviewComments } = useReviewComments(owner, repo, number)

  const reviewWithComments: Review[] =
    reviews?.map((review) => ({
      type: "review",
      review,
      comments:
        reviewComments?.filter(
          (comment) => comment.pull_request_review_id === review.id,
        ) ?? [],
    })) ?? []

  const reviewOrComments: Array<Review | Comment> = [
    ...(comments ?? []).map((comment) => ({
      ...comment,
      type: "comment" as const,
    })),
    ...(reviewWithComments ?? []),
  ].sort((a, b) => {
    const dateA = new Date(
      a.type === "comment" ? a.created_at : (a.review.submitted_at ?? ""),
    ).getTime()
    const dateB = new Date(
      b.type === "comment" ? b.created_at : (b.review.submitted_at ?? ""),
    ).getTime()

    return dateA - dateB
  })

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
        {reviewOrComments.map((item) =>
          item.type === "comment" ? (
            <CommentItem key={`comment-${item.id}`} comment={item} />
          ) : (
            <Review key={`review-${item.review.id}`} review={item} />
          ),
        )}
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
