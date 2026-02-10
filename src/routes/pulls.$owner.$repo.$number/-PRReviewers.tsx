import { CheckCircle, Clock, MessageSquare } from "lucide-react"

type ReviewStatus = "approved" | "changes_requested" | "pending" | "commented"

type MockReviewer = {
  username: string
  avatarInitials: string
  status: ReviewStatus
}

const MOCK_REVIEWERS: MockReviewer[] = [
  {
    username: "john-doe",
    avatarInitials: "JD",
    status: "approved",
  },
  {
    username: "alice-smith",
    avatarInitials: "AS",
    status: "changes_requested",
  },
  {
    username: "bob-jones",
    avatarInitials: "BJ",
    status: "pending",
  },
]

export function PRReviewers() {
  return (
    <div className="rounded-lg border bg-card">
      <div className="p-4 border-b">
        <h3 className="text-sm font-medium">
          Reviewers ({MOCK_REVIEWERS.length})
        </h3>
      </div>
      <div className="p-4 space-y-3">
        {MOCK_REVIEWERS.map((reviewer) => (
          <ReviewerItem key={reviewer.username} reviewer={reviewer} />
        ))}
      </div>
    </div>
  )
}

function ReviewerItem({ reviewer }: { reviewer: MockReviewer }) {
  const { icon: Icon, color, label } = getReviewStatusInfo(reviewer.status)

  return (
    <div className="flex items-center gap-3">
      <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0">
        {reviewer.avatarInitials}
      </div>
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
