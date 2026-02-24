import {
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Pencil,
  Reply,
  Undo2,
} from "lucide-react"
import { useState } from "react"

import type { MaterializedReply, PortedComment } from "@/bindings"
import { InlineCommentForm } from "@/components/InlineCommentForm"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { compareFilePaths } from "@/lib/fileTree"
import { formatRelativeTime } from "@/lib/timeUtils"

import { useLocalCommentMutations } from "../-hooks/useLocalCommentMutations"
import { useLocalComments } from "../-hooks/useLocalComments"

type LocalCommentsSidebarProps = {
  localDir: string
  changeId: string
  sha: string
}

export function LocalCommentsSidebar({
  localDir,
  changeId,
  sha,
}: LocalCommentsSidebarProps) {
  const { data: fileCommentsList } = useLocalComments(localDir, changeId, sha)
  const mutations = useLocalCommentMutations(localDir, changeId, sha)

  const sortedFileComments = (fileCommentsList ?? [])
    .filter((fc) => fc.comments.length > 0)
    .sort(compareFilePaths((fc) => fc.file_path))

  const totalComments = sortedFileComments.reduce(
    (sum, fc) =>
      sum + fc.comments.reduce((s, pc) => s + 1 + pc.comment.replies.length, 0),
    0,
  )

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Comments</h2>
          {totalComments > 0 && (
            <Badge variant="secondary">{totalComments}</Badge>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {totalComments === 0 ? (
          <div className="p-4">
            <p className="text-xs text-muted-foreground">
              No comments for this commit. Select lines in the diff and press{" "}
              <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">c</kbd>{" "}
              to add one.
            </p>
          </div>
        ) : (
          <div className="p-4 space-y-3">
            {sortedFileComments.map((fc) => (
              <FileSection
                key={fc.file_path}
                filePath={fc.file_path}
                portedComments={fc.comments}
                mutations={mutations}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// ---------- File Section ----------

type Mutations = ReturnType<typeof useLocalCommentMutations>

function FileSection({
  filePath,
  portedComments,
  mutations,
}: {
  filePath: string
  portedComments: PortedComment[]
  mutations: Mutations
}) {
  const [isOpen, setIsOpen] = useState(true)

  const totalCount = portedComments.reduce(
    (sum, pc) => sum + 1 + pc.comment.replies.length,
    0,
  )

  // Sort by line number (use ported_line if available, fall back to original)
  const sorted = [...portedComments].sort(
    (a, b) =>
      (a.ported_line ?? a.comment.line) - (b.ported_line ?? b.comment.line),
  )

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger className="flex items-center gap-2 w-full text-left hover:bg-muted/50 p-2 rounded transition-colors">
        {isOpen ? (
          <ChevronDown className="w-4 h-4 shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 shrink-0" />
        )}
        <span className="text-xs font-medium truncate flex-1">{filePath}</span>
        <Badge variant="secondary" className="shrink-0">
          {totalCount}
        </Badge>
      </CollapsibleTrigger>

      <CollapsibleContent className="mt-2 ml-6 space-y-2">
        {sorted.map((pc) => (
          <CommentThread
            key={pc.comment.id}
            portedComment={pc}
            filePath={filePath}
            mutations={mutations}
          />
        ))}
      </CollapsibleContent>
    </Collapsible>
  )
}

// ---------- Comment Thread ----------

function CommentThread({
  portedComment,
  filePath,
  mutations,
}: {
  portedComment: PortedComment
  filePath: string
  mutations: Mutations
}) {
  const { comment, ported_line, ported_start_line, is_ported } = portedComment
  const [isReplying, setIsReplying] = useState(false)
  const [isEditing, setIsEditing] = useState(false)

  const displayLine = ported_line ?? comment.line
  const displayStartLine = ported_start_line ?? comment.start_line

  const handleReply = async (body: string) => {
    await mutations.replyToComment.mutateAsync({
      filePath,
      parentCommentId: comment.id,
      body,
    })
    setIsReplying(false)
  }

  const handleEdit = async (body: string) => {
    await mutations.editComment.mutateAsync({
      filePath,
      commentId: comment.id,
      body,
    })
    setIsEditing(false)
  }

  const handleToggleResolve = () => {
    if (comment.resolved) {
      mutations.unresolveComment.mutate({ filePath, commentId: comment.id })
    } else {
      mutations.resolveComment.mutate({ filePath, commentId: comment.id })
    }
  }

  return (
    <div className="rounded-lg border bg-card">
      {/* Root Comment */}
      <div className="p-3">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-xs text-muted-foreground">
            {formatRelativeTime(comment.created_at)}
          </span>
          {comment.edit_count > 0 && (
            <span className="text-xs text-muted-foreground italic">
              (edited)
            </span>
          )}
          <div className="ml-auto flex items-center gap-1">
            {is_ported && (
              <Badge variant="outline" className="text-[10px] px-1 py-0">
                ported
              </Badge>
            )}
            <span className="text-xs text-muted-foreground">
              {displayStartLine != null && displayStartLine !== displayLine
                ? `L${displayStartLine}-${displayLine}`
                : `L${displayLine}`}
            </span>
          </div>
        </div>

        {isEditing ? (
          <InlineCommentForm
            onSubmit={handleEdit}
            onCancel={() => setIsEditing(false)}
            placeholder="Edit comment..."
          />
        ) : (
          <MarkdownContent>{comment.body}</MarkdownContent>
        )}
      </div>

      {/* Replies */}
      {comment.replies.map((reply) => (
        <ReplyItem
          key={reply.id}
          reply={reply}
          filePath={filePath}
          mutations={mutations}
        />
      ))}

      {/* Actions bar */}
      <div className="border-t p-2 flex items-center gap-1">
        <Button
          variant="ghost"
          size="xs"
          onClick={handleToggleResolve}
          className="text-muted-foreground"
        >
          {comment.resolved ? (
            <>
              <Undo2 className="w-3 h-3" />
              Unresolve
            </>
          ) : (
            <>
              <CheckCircle2 className="w-3 h-3" />
              Resolve
            </>
          )}
        </Button>
        {!isEditing && (
          <Button
            variant="ghost"
            size="xs"
            onClick={() => setIsEditing(true)}
            className="text-muted-foreground"
          >
            <Pencil className="w-3 h-3" />
            Edit
          </Button>
        )}
        <Button
          variant="ghost"
          size="xs"
          onClick={() => setIsReplying(true)}
          className="text-muted-foreground ml-auto"
        >
          <Reply className="w-3 h-3" />
          Reply
        </Button>
      </div>

      {/* Reply form */}
      {isReplying && (
        <div className="border-t">
          <InlineCommentForm
            onSubmit={handleReply}
            onCancel={() => setIsReplying(false)}
            placeholder="Write a reply..."
          />
        </div>
      )}

      {/* Resolved indicator */}
      {comment.resolved && (
        <div className="border-t px-3 py-1.5 bg-muted/30 flex items-center gap-1.5 text-xs text-muted-foreground">
          <Check className="w-3 h-3" />
          Resolved
        </div>
      )}
    </div>
  )
}

// ---------- Reply Item ----------

function ReplyItem({
  reply,
  filePath,
  mutations,
}: {
  reply: MaterializedReply
  filePath: string
  mutations: Mutations
}) {
  const [isEditing, setIsEditing] = useState(false)

  const handleEdit = async (body: string) => {
    await mutations.editComment.mutateAsync({
      filePath,
      commentId: reply.id,
      body,
    })
    setIsEditing(false)
  }

  return (
    <div className="border-t p-3">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-xs text-muted-foreground">
          {formatRelativeTime(reply.created_at)}
        </span>
        {reply.edit_count > 0 && (
          <span className="text-xs text-muted-foreground italic">(edited)</span>
        )}
        {!isEditing && (
          <Button
            variant="ghost"
            size="xs"
            onClick={() => setIsEditing(true)}
            className="text-muted-foreground ml-auto h-5 px-1"
          >
            <Pencil className="w-3 h-3" />
          </Button>
        )}
      </div>
      {isEditing ? (
        <InlineCommentForm
          onSubmit={handleEdit}
          onCancel={() => setIsEditing(false)}
          placeholder="Edit reply..."
        />
      ) : (
        <MarkdownContent>{reply.body}</MarkdownContent>
      )}
    </div>
  )
}
