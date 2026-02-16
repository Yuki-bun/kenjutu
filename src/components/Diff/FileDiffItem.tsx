import { useQueryClient } from "@tanstack/react-query"
import { Check, ChevronDown, ChevronRight, Copy } from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"
import { toast } from "sonner"

import { commands, DiffLine, FileEntry } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import {
  PANEL_KEYS,
  softFocusItemInPanel,
  useScrollFocusItem,
} from "@/components/ScrollFocus"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { useRpcMutation, useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"
import { cn } from "@/lib/utils"

import { useDiffContext } from "./CommitDiffSection"
import { getStatusStyle } from "./diffStyles"
import { augmentHunks, buildDiffElements, HunkGap } from "./hunkGaps"
import { ExpandDirection, SplitDiff } from "./SplitDiff"
import { UnifiedDiff } from "./UnifiedDiff"

const EXPAND_LINES_COUNT = 20

const LARGE_FILE_THRESHOLD = 500
const GENERATED_FILE_PATTERNS = [
  "pnpm-lock.yaml",
  "package-lock.json",
  "yarn.lock",
]

function shouldAutoCollapse(file: FileEntry): boolean {
  const totalChanges = file.additions + file.deletions
  if (totalChanges > LARGE_FILE_THRESHOLD) return true

  const filePath = file.newPath || file.oldPath || ""
  const fileName = filePath.split("/").pop() ?? ""
  return GENERATED_FILE_PATTERNS.includes(fileName)
}

export type PRCommentContext = {
  onCreateComment: (params: {
    body: string
    path: string
    line: number
    side: "LEFT" | "RIGHT"
    commitId: string
    startLine?: number
    startSide?: "LEFT" | "RIGHT"
  }) => Promise<void>
  isCommentPending: boolean
}

export type CommentLineState = {
  line: number
  side: "LEFT" | "RIGHT"
  startLine?: number
  startSide?: "LEFT" | "RIGHT"
} | null

export function FileDiffItem({
  file,
  prComment,
  InlineCommentForm,
}: {
  file: FileEntry
  prComment?: PRCommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
}) {
  const { localDir, commitSha, changeId } = useDiffContext()
  const [isOpen, setIsOpen] = useState(
    !file.isReviewed && !shouldAutoCollapse(file),
  )
  const queryClient = useQueryClient()

  const onFocus = () => {
    softFocusItemInPanel(
      PANEL_KEYS.fileTree,
      file.newPath || file.oldPath || "",
    )
  }

  const { ref, isFocused, scrollIntoView } = useScrollFocusItem<HTMLDivElement>(
    file.newPath || file.oldPath || "",
    { onFocus },
  )

  const toggleMutation = useRpcMutation({
    mutationFn: async (isReviewed: boolean) => {
      if (!changeId) {
        throw new Error("Cannot mark as reviewed: no change ID")
      }
      const filePath = file.newPath || file.oldPath || ""
      return await commands.toggleFileReviewed(
        localDir,
        changeId,
        filePath,
        file.patchId!,
        isReviewed,
      )
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.commitFileList(localDir, commitSha),
      })
    },
  })

  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!file.patchId || !changeId) return
    const isReviewed = e.target.checked
    toggleMutation.mutate(isReviewed)
    if (isReviewed) {
      onClose()
    }
  }

  const handleOpenChange = (isOpen: boolean) => {
    if (isOpen) {
      setIsOpen(true)
    } else {
      onClose()
    }
  }

  const onClose = () => {
    setTimeout(scrollIntoView, 0)
    setIsOpen(false)
  }

  useHotkeys(
    "enter",
    () => {
      const newIsReviewed = !file.isReviewed
      toggleMutation.mutate(newIsReviewed)
      if (newIsReviewed) {
        onClose()
      }
    },
    {
      enabled: isFocused,
    },
  )
  useHotkeys(
    "o",
    () => {
      if (isOpen) {
        onClose()
      } else {
        setIsOpen(!isOpen)
      }
    },
    {
      enabled: isFocused,
    },
  )

  const displayPath =
    file.status === "renamed"
      ? `${file.oldPath} => ${file.newPath}`
      : file.newPath || file.oldPath || "unknown"

  const { bgColor, textColor, label } = getStatusStyle(file.status)
  // Can only review if we have both patchId and changeId
  const canBeReviewed =
    file.patchId !== null && file.patchId !== undefined && changeId !== null

  const [copied, setCopied] = useState(false)

  const copyFilePath = useCallback(() => {
    navigator.clipboard.writeText(file.newPath || file.oldPath || "")
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }, [file.newPath, file.oldPath])

  const handleCopyFilePath = (e: React.MouseEvent) => {
    e.stopPropagation()
    copyFilePath()
  }

  useHotkeys("c", () => copyFilePath(), { enabled: isFocused })

  return (
    <Collapsible
      ref={ref}
      tabIndex={0}
      className="border rounded-lg focusKey"
      open={isOpen}
      onOpenChange={handleOpenChange}
    >
      {/* Sticky File Header */}
      <div className="sticky top-0 z-20 flex items-center justify-between p-3 bg-muted rounded-t-lg border-b">
        <div className="flex items-center gap-3 flex-1 min-w-0">
          {/* Checkbox for reviewed status */}
          {canBeReviewed && (
            <input
              type="checkbox"
              tabIndex={-1}
              checked={file.isReviewed || false}
              onChange={handleCheckboxChange}
              onClick={(e) => e.stopPropagation()}
              disabled={toggleMutation.isPending}
              className="h-4 w-4 rounded border-gray-300 cursor-pointer"
              title="Mark as reviewed"
            />
          )}

          {/* Collapsible trigger */}
          <CollapsibleTrigger asChild>
            <div className="flex items-center gap-3 flex-1 min-w-0 cursor-pointer hover:bg-muted/50">
              <div className="shrink-0">
                {isOpen ? (
                  <ChevronDown className="w-4 h-4" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
              </div>
              <span
                className={cn(
                  "text-xs font-semibold uppercase px-2 py-1 rounded shrink-0",
                  bgColor,
                  textColor,
                )}
              >
                {label}
              </span>
              <span className="font-mono text-sm truncate" title={displayPath}>
                {displayPath}
              </span>
              {copied ? (
                <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400 shrink-0">
                  <Check className="w-4 h-4" />
                  Copied!
                </span>
              ) : (
                <Copy
                  className="w-4 h-4 shrink-0 text-muted-foreground hover:text-foreground cursor-pointer"
                  onClick={handleCopyFilePath}
                />
              )}
            </div>
          </CollapsibleTrigger>
        </div>
        <div className="flex items-center gap-3 text-xs shrink-0 ml-2">
          <span className="text-green-600 dark:text-green-400">
            +{file.additions}
          </span>
          <span className="text-red-600 dark:text-red-400">
            -{file.deletions}
          </span>
        </div>
      </div>

      {/* File Content - Lazy loaded */}
      <CollapsibleContent>
        <div className="overflow-x-auto rounded-b-lg">
          {file.isBinary ? (
            <div className="p-4 text-center text-muted-foreground text-sm">
              Binary file changed
            </div>
          ) : (
            <LazyFileDiff
              filePath={file.newPath || file.oldPath || ""}
              oldPath={
                file.status === "renamed"
                  ? (file.oldPath ?? undefined)
                  : undefined
              }
              prComment={prComment}
              InlineCommentForm={InlineCommentForm}
            />
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  )
}

export type InlineCommentFormProps = {
  onSubmit: (body: string) => void
  onCancel: () => void
  isPending: boolean
}

function LazyFileDiff({
  filePath,
  oldPath,
  prComment,
  InlineCommentForm,
}: {
  filePath: string
  oldPath?: string
  prComment?: PRCommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
}) {
  const { localDir, commitSha, diffViewMode } = useDiffContext()
  const [commentLine, setCommentLine] = useState<CommentLineState>(null)
  const [fetchedContextLines, setFetchedContextLines] = useState<
    Map<number, DiffLine>
  >(new Map())

  const { data, error, isLoading } = useRpcQuery({
    queryKey: queryKeys.fileDiff(localDir, commitSha, filePath, oldPath),
    queryFn: () =>
      commands.getFileDiff(localDir, commitSha, filePath, oldPath ?? null),
    staleTime: Infinity,
  })

  const augmentedHunks = useMemo(
    () =>
      data
        ? augmentHunks(data.hunks, fetchedContextLines, data.newFileLines)
        : [],
    [data, fetchedContextLines],
  )

  const elements = useMemo(
    () => (data ? buildDiffElements(augmentedHunks, data.newFileLines) : []),
    [augmentedHunks, data],
  )

  const handleExpandGap = useCallback(
    async (gap: HunkGap, direction: ExpandDirection) => {
      let fetchStart: number
      let fetchEnd: number

      if (direction === "all") {
        fetchStart = gap.newStart
        fetchEnd = gap.newEnd
      } else if (direction === "down") {
        fetchStart = gap.newStart
        fetchEnd = Math.min(gap.newStart + EXPAND_LINES_COUNT - 1, gap.newEnd)
      } else {
        fetchEnd = gap.newEnd
        fetchStart = Math.max(gap.newEnd - EXPAND_LINES_COUNT + 1, gap.newStart)
      }

      if (fetchStart > fetchEnd) return

      const oldStartLine = gap.oldStart + (fetchStart - gap.newStart)

      const result = await commands.getContextLines(
        localDir,
        commitSha,
        filePath,
        fetchStart,
        fetchEnd,
        oldStartLine,
      )

      if (result.status === "error") {
        toast.error("Failed to expand context lines")
        return
      }

      setFetchedContextLines((prev) => {
        const next = new Map(prev)
        for (const line of result.data) {
          if (line.newLineno != null) {
            next.set(line.newLineno, line)
          }
        }
        return next
      })
    },
    [localDir, commitSha, filePath],
  )

  const [isDragging, setIsDragging] = useState(false)
  const dragRef = useRef<{
    startLine: number
    side: "LEFT" | "RIGHT"
  } | null>(null)

  const handleLineDragStart = prComment
    ? (line: number, side: "LEFT" | "RIGHT") => {
        dragRef.current = { startLine: line, side }
        setIsDragging(true)
        setCommentLine({ line, side })
      }
    : undefined

  const handleLineDragEnter = prComment
    ? (line: number, side: "LEFT" | "RIGHT") => {
        if (!dragRef.current || dragRef.current.side !== side) return
        const startLine = Math.min(dragRef.current.startLine, line)
        const endLine = Math.max(dragRef.current.startLine, line)
        setCommentLine(
          startLine === endLine
            ? { line: endLine, side }
            : { line: endLine, side, startLine, startSide: side },
        )
      }
    : undefined

  const handleLineDragEnd = prComment
    ? () => {
        dragRef.current = null
        setIsDragging(false)
      }
    : undefined

  // End drag on mouseup anywhere (in case user releases outside gutter)
  useEffect(() => {
    const onMouseUp = () => {
      if (dragRef.current) {
        dragRef.current = null
        setIsDragging(false)
      }
    }
    document.addEventListener("mouseup", onMouseUp)
    return () => document.removeEventListener("mouseup", onMouseUp)
  }, [])

  const handleSubmitComment = async (body: string) => {
    if (!prComment || !commentLine) return
    prComment.onCreateComment({
      body,
      path: filePath,
      line: commentLine.line,
      side: commentLine.side,
      commitId: commitSha,
      startLine: commentLine.startLine,
      startSide: commentLine.startSide,
    })
    setCommentLine(null)
  }

  const commentForm =
    InlineCommentForm && commentLine && !isDragging ? (
      <InlineCommentForm
        onSubmit={handleSubmitComment}
        onCancel={() => setCommentLine(null)}
        isPending={prComment?.isCommentPending ?? false}
      />
    ) : null

  if (isLoading) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        Loading diff...
      </div>
    )
  }

  if (error) {
    return (
      <div className="p-4">
        <ErrorDisplay error={error} />
      </div>
    )
  }

  if (!data) {
    return null
  }

  if (data.hunks.length === 0) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        No content changes
      </div>
    )
  }

  const sharedProps = {
    elements,
    onExpandGap: handleExpandGap,
    commentLine,
    onLineDragStart: handleLineDragStart,
    onLineDragEnter: handleLineDragEnter,
    onLineDragEnd: handleLineDragEnd,
    commentForm,
  }

  if (diffViewMode === "split") {
    return <SplitDiff {...sharedProps} />
  }

  return <UnifiedDiff {...sharedProps} />
}
