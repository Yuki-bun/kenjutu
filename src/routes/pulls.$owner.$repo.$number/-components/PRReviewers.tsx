import { CheckCircle, Clock, MessageSquare } from "lucide-react"

import {
  type Reviewer,
  usePullRequestReviews,
} from "../-hooks/usePullRequestReviews"

type ReviewStatus = "approved" | "changes_requested" | "pending" | "commented"

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
  const {
    data: reviewers,
    isLoading,
    error,
  } = usePullRequestReviews(owner, repo, number)

  return (
    <div className="rounded-lg border bg-card">
      <div className="p-4 border-b">
        <h3 className="text-sm font-medium">
          Reviewers {reviewers && `(${reviewers.length})`}
        </h3>
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
        {reviewers && reviewers.length === 0 && (
          <div className="text-sm text-muted-foreground">
            No reviewers assigned
          </div>
        )}
        {reviewers &&
          reviewers.length > 0 &&
          reviewers.map((reviewer) => (
            <ReviewerItem key={reviewer.username} reviewer={reviewer} />
          ))}
      </div>
    </div>
  )
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
