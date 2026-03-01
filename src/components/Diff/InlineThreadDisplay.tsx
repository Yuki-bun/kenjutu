import { CheckCircle2, GitCommitHorizontal, Reply } from "lucide-react"
import { useState } from "react"

import { InlineCommentForm } from "@/components/InlineCommentForm"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/lib/timeUtils"

import type { InlineCommentUser, InlineReply, InlineThread } from "./types"

function UserAvatar({ user }: { user: InlineCommentUser }) {
  return (
    <div className="w-5 h-5 rounded-full bg-muted flex items-center justify-center text-[10px] font-medium shrink-0 overflow-hidden">
      {user.avatarUrl ? (
        <img
          src={user.avatarUrl}
          alt={user.login}
          className="w-full h-full object-cover"
        />
      ) : (
        user.login[0].toUpperCase()
      )}
    </div>
  )
}

function CommentHeader({
  user,
  createdAt,
}: {
  user?: InlineCommentUser
  createdAt: string
}) {
  return (
    <div className="flex items-center gap-1.5 mb-1">
      {user && (
        <>
          <UserAvatar user={user} />
          <span className="font-semibold text-xs">{user.login}</span>
        </>
      )}
      <span className="text-[10px] text-muted-foreground">
        {formatRelativeTime(createdAt)}
      </span>
    </div>
  )
}

function ReplyItem({ reply }: { reply: InlineReply }) {
  return (
    <div className="border-t px-3 py-2">
      <CommentHeader user={reply.user} createdAt={reply.createdAt} />
      <MarkdownContent className="text-xs [&_p]:text-xs">
        {reply.body}
      </MarkdownContent>
    </div>
  )
}

export function InlineThreadDisplay({
  thread,
  onReply,
}: {
  thread: InlineThread
  onReply?: (threadId: string, body: string) => void
}) {
  const [isReplying, setIsReplying] = useState(false)

  const handleReply = (body: string) => {
    onReply?.(thread.id, body)
    setIsReplying(false)
  }

  return (
    <div className="rounded border bg-card text-card-foreground">
      {/* Root comment */}
      <div className="px-3 py-2">
        <div className="flex items-center gap-1.5 mb-1">
          {thread.user && (
            <>
              <UserAvatar user={thread.user} />
              <span className="font-semibold text-xs">{thread.user.login}</span>
            </>
          )}
          <span className="text-[10px] text-muted-foreground">
            {formatRelativeTime(thread.createdAt)}
          </span>
          <div className="ml-auto flex items-center gap-1">
            {thread.isPorted && (
              <Badge
                variant="outline"
                className="text-[10px] px-1 py-0 h-4 gap-0.5"
              >
                <GitCommitHorizontal className="w-2.5 h-2.5" />
                ported
              </Badge>
            )}
            {thread.resolved && (
              <Badge
                variant="outline"
                className="text-[10px] px-1 py-0 h-4 gap-0.5 text-green-600 border-green-600/30"
              >
                <CheckCircle2 className="w-2.5 h-2.5" />
                resolved
              </Badge>
            )}
          </div>
        </div>
        <MarkdownContent className="text-xs [&_p]:text-xs">
          {thread.body}
        </MarkdownContent>
      </div>

      {/* Replies */}
      {thread.replies.map((reply) => (
        <ReplyItem key={reply.id} reply={reply} />
      ))}

      {/* Reply action */}
      {onReply &&
        (isReplying ? (
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
              className="w-full text-muted-foreground text-[10px]"
            >
              <Reply className="w-3 h-3" />
              Reply
            </Button>
          </div>
        ))}
    </div>
  )
}
