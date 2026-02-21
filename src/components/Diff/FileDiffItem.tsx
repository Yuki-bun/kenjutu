import { useQueryClient } from "@tanstack/react-query"
import { Check, ChevronDown, ChevronRight, Copy } from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"
import { toast } from "sonner"

import { commands, DiffLine, FileEntry, HunkId, ReviewStatus } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import {
  PANEL_KEYS,
  usePaneContext,
  usePaneItem,
  usePaneManager,
} from "@/components/Pane"
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
import { DualDiff, DualDiffPanel } from "./DualDiff"
import { augmentHunks, buildDiffElements, HunkGap } from "./hunkGaps"
import { ExpandDirection, SplitDiff } from "./SplitDiff"
import { UnifiedDiff } from "./UnifiedDiff"
import { LineModeControl, LineModeState, useLineMode } from "./useLineMode"

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
  const { softFocusPaneItem } = usePaneManager()
  const { setSuppressNavigation } = usePaneContext()
  const [isOpen, setIsOpen] = useState(
    file.reviewStatus !== "reviewed" &&
      file.reviewStatus !== "reviewedReverted" &&
      !shouldAutoCollapse(file),
  )
  const queryClient = useQueryClient()
  const checkboxRef = useRef<HTMLInputElement>(null)

  const onFocus = () => {
    softFocusPaneItem(PANEL_KEYS.fileTree, file.newPath || file.oldPath || "")
  }

  const { ref, isFocused, scrollIntoView } = usePaneItem<HTMLDivElement>(
    file.newPath || file.oldPath || "",
  )

  const toggleMutation = useRpcMutation({
    mutationFn: async (isReviewed: boolean) => {
      const filePath = file.newPath || file.oldPath || ""
      return await commands.toggleFileReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        file.status === "renamed" ? file.oldPath : null,
        isReviewed,
      )
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.commitFileList(localDir, commitSha),
      })
      const filePath = file.newPath || file.oldPath || ""
      const oldPath =
        file.status === "renamed" ? (file.oldPath ?? undefined) : undefined
      queryClient.invalidateQueries({
        queryKey: queryKeys.partialReviewDiffs(
          localDir,
          changeId,
          commitSha,
          filePath,
          oldPath,
        ),
      })
    },
  })

  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!changeId) return
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

  const [lineModeState, setLineModeState] = useState<LineModeState | null>(null)
  const isLineModeActive = lineModeState !== null

  const enterLineMode = useCallback(() => {
    setIsOpen(true)
    setSuppressNavigation(true)
    setLineModeState({
      cursorIndex: 0,
      selection: { isSelecting: false },
    })
  }, [setSuppressNavigation])

  const exitLineMode = useCallback(() => {
    setSuppressNavigation(false)
    setLineModeState(null)
  }, [setSuppressNavigation])

  useEffect(() => {
    if (!isFocused && isLineModeActive) {
      exitLineMode()
    }
  }, [isFocused, isLineModeActive, exitLineMode])

  useEffect(() => {
    return () => setSuppressNavigation(false)
  }, [setSuppressNavigation])

  useHotkeys(
    "space",
    (e) => {
      e.preventDefault()
      const newIsReviewed = file.reviewStatus !== "reviewed"
      toggleMutation.mutate(newIsReviewed)
      if (newIsReviewed) {
        onClose()
      }
    },
    {
      enabled: isFocused && !isLineModeActive,
    },
  )
  useHotkeys("enter", () => enterLineMode(), {
    enabled: isFocused && !isLineModeActive,
  })
  useHotkeys(
    "o",
    () => {
      if (isOpen) {
        onClose()
      } else {
        setIsOpen(true)
      }
    },
    {
      enabled: isFocused && !isLineModeActive,
    },
  )

  const displayPath =
    file.status === "renamed"
      ? `${file.oldPath} => ${file.newPath}`
      : file.newPath || file.oldPath || "unknown"

  const { bgColor, textColor, label } = getStatusStyle(file.status)

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

  useHotkeys("c", () => copyFilePath(), {
    enabled: isFocused && !isLineModeActive,
  })

  return (
    <Collapsible
      ref={ref}
      tabIndex={0}
      className="border rounded-lg focusKey"
      open={isOpen}
      onOpenChange={handleOpenChange}
      onFocus={onFocus}
    >
      {/* Sticky File Header */}
      <div className="sticky top-0 z-20 flex items-center justify-between p-3 bg-muted rounded-t-lg border-b">
        <div className="flex items-center gap-3 flex-1 min-w-0">
          {/* Checkbox for reviewed status */}
          <input
            ref={checkboxRef}
            type="checkbox"
            tabIndex={-1}
            checked={file.reviewStatus === "reviewed"}
            onChange={handleCheckboxChange}
            onClick={(e) => e.stopPropagation()}
            disabled={toggleMutation.isPending}
            className="h-4 w-4 rounded border-gray-300 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
            title={
              file.reviewStatus === "reviewedReverted"
                ? "Change was reverted — nothing to review"
                : "Mark as reviewed"
            }
          />

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
              {file.reviewStatus === "reviewedReverted" && (
                <span className="text-xs text-red-600 dark:text-red-400 shrink-0">
                  Reverted
                </span>
              )}
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
          ) : file.reviewStatus !== "reviewedReverted" ? (
            <LazyFileDiff
              filePath={file.newPath || file.oldPath || ""}
              oldPath={
                file.status === "renamed"
                  ? (file.oldPath ?? undefined)
                  : undefined
              }
              reviewStatus={file.reviewStatus}
              prComment={prComment}
              InlineCommentForm={InlineCommentForm}
              lineMode={{
                state: lineModeState,
                setState: setLineModeState,
                onExit: exitLineMode,
              }}
            />
          ) : (
            <p>Changes were reverted after marking as reviewed</p>
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  )
}

export type InlineCommentFormProps = {
  onSubmit: (body: string) => void
  onCancel: () => void
}

function LazyFileDiff({
  filePath,
  oldPath,
  reviewStatus,
  prComment,
  InlineCommentForm,
  lineMode,
}: {
  filePath: string
  oldPath?: string
  reviewStatus: ReviewStatus
  prComment?: PRCommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
  lineMode: LineModeControl
}) {
  const { localDir, commitSha, changeId, diffViewMode } = useDiffContext()
  const queryClient = useQueryClient()
  const diffContainerRef = useRef<HTMLDivElement>(null)
  const [commentLine, setCommentLine] = useState<CommentLineState>(null)
  const [fetchedContextLines, setFetchedContextLines] = useState<
    Map<number, DiffLine>
  >(new Map())

  const isPartial =
    reviewStatus === "partiallyReviewed" || reviewStatus === "reviewedReverted"

  const { data, error, isLoading } = useRpcQuery({
    queryKey: queryKeys.fileDiff(localDir, commitSha, filePath, oldPath),
    queryFn: () =>
      commands.getFileDiff(localDir, commitSha, filePath, oldPath ?? null),
    staleTime: Infinity,
    enabled: !isPartial,
  })

  const {
    data: partialData,
    error: partialError,
    isLoading: partialLoading,
  } = useRpcQuery({
    queryKey: queryKeys.partialReviewDiffs(
      localDir,
      changeId,
      commitSha,
      filePath,
      oldPath,
    ),
    queryFn: () =>
      commands.getPartialReviewDiffs(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
      ),
    enabled: isPartial,
  })

  const remainingElements = useMemo(
    () =>
      partialData
        ? buildDiffElements(
            partialData.remaining.hunks,
            partialData.remaining.newFileLines,
          )
        : [],
    [partialData],
  )

  const reviewedElements = useMemo(
    () =>
      partialData
        ? buildDiffElements(
            partialData.reviewed.hunks,
            partialData.reviewed.newFileLines,
          )
        : [],
    [partialData],
  )

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

  const handleLineComment = useCallback(
    (comment: NonNullable<CommentLineState>) => {
      setCommentLine(comment)
      lineMode.onExit()
    },
    [lineMode],
  )

  const invalidateAfterHunkMark = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: queryKeys.commitFileList(localDir, commitSha),
    })
    queryClient.invalidateQueries({
      queryKey: queryKeys.fileDiff(localDir, commitSha, filePath, oldPath),
    })
    queryClient.invalidateQueries({
      queryKey: queryKeys.partialReviewDiffs(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath,
      ),
    })
  }, [queryClient, localDir, commitSha, filePath, oldPath, changeId])

  const markRegionMutation = useRpcMutation({
    mutationFn: async (region: HunkId) => {
      if (!changeId) {
        throw new Error("Cannot mark region: no change ID")
      }
      return await commands.markHunkReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterHunkMark,
  })

  const unmarkRegionMutation = useRpcMutation({
    mutationFn: async (region: HunkId) => {
      return await commands.unmarkHunkReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterHunkMark,
  })

  const handleMarkRegion = useCallback(
    (region: HunkId) => {
      if (reviewStatus === "reviewed") {
        unmarkRegionMutation.mutate(region)
      } else {
        markRegionMutation.mutate(region)
      }
    },
    [reviewStatus, markRegionMutation, unmarkRegionMutation],
  )

  const handleDualMarkRegion = useCallback(
    (region: HunkId, panel: DualDiffPanel) => {
      if (panel === "remaining") {
        markRegionMutation.mutate(region)
      } else {
        unmarkRegionMutation.mutate(region)
      }
    },
    [markRegionMutation, unmarkRegionMutation],
  )

  const { lineCursor } = useLineMode({
    elements,
    diffViewMode,
    containerRef: diffContainerRef,
    onComment: prComment && InlineCommentForm ? handleLineComment : undefined,
    onMarkRegion: !isPartial && changeId ? handleMarkRegion : undefined,
    ...lineMode,
    state: isPartial ? null : lineMode.state,
  })

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

  const handleSubmitComment = (body: string) => {
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
      />
    ) : null

  const activeLoading = isPartial ? partialLoading : isLoading
  const activeError = isPartial ? partialError : error

  if (activeLoading) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        Loading diff...
      </div>
    )
  }

  if (activeError) {
    return (
      <div className="p-4">
        <ErrorDisplay error={activeError} />
      </div>
    )
  }

  if (isPartial) {
    if (!partialData) return null
    return (
      <div ref={diffContainerRef}>
        <DualDiff
          remainingElements={remainingElements}
          reviewedElements={reviewedElements}
          lineMode={changeId ? lineMode : undefined}
          onMarkRegion={changeId ? handleDualMarkRegion : undefined}
        />
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
    lineCursor,
  }

  return (
    <div ref={diffContainerRef}>
      {diffViewMode === "split" ? (
        <SplitDiff {...sharedProps} />
      ) : (
        <UnifiedDiff {...sharedProps} />
      )}
    </div>
  )
}
