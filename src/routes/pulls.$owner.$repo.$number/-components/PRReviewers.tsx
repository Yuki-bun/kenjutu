import { CheckCircle, Clock, MessageSquare } from "lucide-react"

import { usePullRequestReviews } from "../-hooks/usePullRequestReviews"

type PRReviewersProps = {
  owner: string
  repo: string
  number: number
}

function getUserInitials(username: string): string {
  return username
    .split(/[-_]/)
    .map((part) => part[0])
    .join("")
    .toUpperCase()
    .slice(0, 2)
}

export function PRReviewers({ owner, repo, number }: PRReviewersProps) {
  const { data, isLoading, error } = usePullRequestReviews(owner, repo, number)

  const reviewers =
    data?.reduce<Reviewer[]>((reviews, review) => {
      const reviewer = review.user
      if (!reviewer) {
        return reviews
      }
      const olderReview = reviews.find(
        (olderReview) => olderReview.userId === reviewer.id,
      )
      if (olderReview) {
        olderReview.status = mapGitHubStateToStatus(review.state)
      } else {
        reviews.push({
          status: mapGitHubStateToStatus(review.state),
          userId: reviewer.id,
          username: reviewer.login,
          avatarUrl: reviewer.avatar_url,
        })
      }
      return reviews
    }, []) ?? []

  return (
    <div className="rounded-lg border bg-card">
      <div className="p-4 border-b">
        <h3 className="text-sm font-medium">Reviewers ({reviewers.length})</h3>
      </div>
      <div className="p-4 space-y-3">
        {isLoading && (
          <div className="text-sm text-muted-foreground">
            Loading reviewers...
          </div>
        )}
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">
            Failed to load reviewers
          </div>
        )}
        {reviewers.length === 0 && (
          <div className="text-sm text-muted-foreground">
            No reviewers assigned
          </div>
        )}
        {reviewers.length > 0 &&
          reviewers.map((reviewer) => (
            <ReviewerItem key={reviewer.username} reviewer={reviewer} />
          ))}
      </div>
    </div>
  )
}

type ReviewStatus = "approved" | "changes_requested" | "pending" | "commented"

type Reviewer = {
  userId: number
  username: string
  avatarUrl?: string
  status: ReviewStatus
}

function ReviewerItem({ reviewer }: { reviewer: Reviewer }) {
  const { icon: Icon, color, label } = getReviewStatusInfo(reviewer.status)

  return (
    <div className="flex items-center gap-3">
      {reviewer.avatarUrl ? (
        <img
          src={reviewer.avatarUrl}
          alt={reviewer.username}
          className="w-8 h-8 rounded-full shrink-0"
        />
      ) : (
        <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0">
          {getUserInitials(reviewer.username)}
        </div>
      )}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium">{reviewer.username}</div>
        <div className={`text-xs flex items-center gap-1 ${color}`}>
          <Icon className="w-3 h-3" />
          <span>{label}</span>
        </div>
      </div>
    </div>
  )
}

function getReviewStatusInfo(status: ReviewStatus) {
  switch (status) {
    case "approved":
      return {
        icon: CheckCircle,
        color: "text-green-600 dark:text-green-400",
        label: "Approved",
      }
    case "changes_requested":
      return {
        icon: MessageSquare,
        color: "text-red-600 dark:text-red-400",
        label: "Changes requested",
      }
    case "pending":
      return {
        icon: Clock,
        color: "text-muted-foreground",
        label: "Review requested",
      }
    case "commented":
      return {
        icon: MessageSquare,
        color: "text-muted-foreground",
        label: "Commented",
      }
  }
}

function mapGitHubStateToStatus(state: string): ReviewStatus {
  switch (state) {
    case "APPROVED":
      return "approved"
    case "CHANGES_REQUESTED":
      return "changes_requested"
    case "COMMENTED":
      return "commented"
    case "DISMISSED":
      return "commented"
    case "PENDING":
      return "pending"
    default:
      return "commented"
  }
}
