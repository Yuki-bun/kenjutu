type MockComment = {
  author: string
  avatarInitials: string
  timestamp: string
  body: string
}

const MOCK_COMMENTS: MockComment[] = [
  {
    author: "octocat",
    avatarInitials: "OC",
    timestamp: "2 hours ago",
    body: "Great work on this feature! The commit-by-commit approach makes it really easy to review.",
  },
  {
    author: "reviewer123",
    avatarInitials: "R1",
    timestamp: "1 hour ago",
    body: "Could you add tests for the error handling in the parseCommit function? Otherwise LGTM!",
  },
  {
    author: "author",
    avatarInitials: "AU",
    timestamp: "30 minutes ago",
    body: "@reviewer123 Added tests in commit abc1234. Let me know if you need anything else!",
  },
]

export function PRComments() {
  return (
    <div className="space-y-4">
      <h3 className="text-sm font-medium">Comments ({MOCK_COMMENTS.length})</h3>
      <div className="space-y-4">
        {MOCK_COMMENTS.map((comment, idx) => (
          <CommentItem key={idx} comment={comment} />
        ))}
      </div>
    </div>
  )
}

function CommentItem({ comment }: { comment: MockComment }) {
  return (
    <div className="flex gap-3">
      <div className="w-10 h-10 rounded-full bg-muted flex items-center justify-center text-sm font-medium shrink-0">
        {comment.avatarInitials}
      </div>
      <div className="flex-1 min-w-0">
        <div className="rounded-lg border bg-card">
          <div className="px-4 py-3 border-b bg-muted/30">
            <div className="flex items-baseline gap-2">
              <span className="text-sm font-semibold">{comment.author}</span>
              <span className="text-xs text-muted-foreground">
                commented {comment.timestamp}
              </span>
            </div>
          </div>
          <div className="px-4 py-3">
            <p className="text-sm whitespace-pre-wrap leading-relaxed">
              {comment.body}
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
