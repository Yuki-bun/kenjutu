import { useHotkey } from "@tanstack/react-hotkeys"
import { keepPreviousData, useQueryClient } from "@tanstack/react-query"
import { Check, ChevronDown, ChevronRight, Copy } from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import { commands, FileEntry } from "@/bindings"
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
import { DualDiff } from "./DualDiff"
import { augmentHunks, buildDiffElements } from "./hunkGaps"
import { SplitDiff } from "./SplitDiff"
import type { CommentContext, InlineCommentFormProps } from "./types"
import { UnifiedDiff } from "./UnifiedDiff"
import { useContextExpansion } from "./useContextExpansion"
import { useHunkReview } from "./useHunkReview"
import { useLineDrag } from "./useLineDrag"
import { LineModeControl, LineModeState, useLineMode } from "./useLineMode"

export type {
  CommentContext,
  CommentLineState,
  InlineCommentFormProps,
} from "./types"

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

export function FileDiffItem({
  file,
  commentContext,
  InlineCommentForm,
  getInlineThreads,
}: {
  file: FileEntry
  commentContext?: CommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
  getInlineThreads?: (
    line: number,
    side: "LEFT" | "RIGHT",
  ) => React.ReactNode | null
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

  const onFocus = () => {
    softFocusPaneItem(PANEL_KEYS.fileTree, file.newPath || file.oldPath || "")
  }

  const { ref, scrollIntoView } = usePaneItem<HTMLDivElement>(
    file.newPath || file.oldPath || "",
    { onBlur: exitLineMode },
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

  useEffect(() => {
    return () => setSuppressNavigation(false)
  }, [setSuppressNavigation])

  useHotkey(
    "Space",
    () => {
      const newIsReviewed = file.reviewStatus !== "reviewed"
      toggleMutation.mutate(newIsReviewed)
      if (newIsReviewed) {
        onClose()
      }
    },
    {
      enabled: !isLineModeActive,
      target: ref,
    },
  )
  useHotkey("Enter", () => enterLineMode(), {
    enabled: !isLineModeActive,
    target: ref,
  })
  useHotkey(
    "O",
    () => {
      if (isOpen) {
        onClose()
      } else {
        setIsOpen(true)
      }
    },
    {
      enabled: !isLineModeActive,
      target: ref,
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

  useHotkey("C", () => copyFilePath(), {
    enabled: !isLineModeActive,
    target: ref,
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
          ) : (
            <LazyFileDiff
              filePath={file.newPath || file.oldPath || ""}
              oldPath={
                file.status === "renamed"
                  ? (file.oldPath ?? undefined)
                  : undefined
              }
              commentContext={commentContext}
              InlineCommentForm={InlineCommentForm}
              getInlineThreads={getInlineThreads}
              lineMode={{
                state: lineModeState,
                setState: setLineModeState,
                onExit: exitLineMode,
              }}
            />
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  )
}

function LazyFileDiff({
  filePath,
  oldPath,
  commentContext,
  InlineCommentForm,
  getInlineThreads,
  lineMode,
}: {
  filePath: string
  oldPath?: string
  commentContext?: CommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
  getInlineThreads?: (
    line: number,
    side: "LEFT" | "RIGHT",
  ) => React.ReactNode | null
  lineMode: LineModeControl
}) {
  const { localDir, commitSha, changeId, diffViewMode } = useDiffContext()
  const diffContainerRef = useRef<HTMLDivElement>(null)

  const { data, error, isLoading } = useRpcQuery({
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
    placeholderData: keepPreviousData,
  })

  const hasRemaining = (data?.remaining.hunks.length ?? 0) > 0
  const hasReviewed = (data?.reviewed.hunks.length ?? 0) > 0
  const isSplit = hasRemaining && hasReviewed

  // Pick which side to show in single-panel mode
  const singleSide: "remaining" | "reviewed" = hasRemaining
    ? "remaining"
    : "reviewed"
  const singleDiff = data
    ? singleSide === "remaining"
      ? data.remaining
      : data.reviewed
    : undefined

  const { fetchedContextLines, handleExpandGap } = useContextExpansion({
    localDir,
    commitSha,
    filePath,
  })

  const { handleDualMarkRegion } = useHunkReview({
    localDir,
    commitSha,
    changeId,
    filePath,
    oldPath,
  })

  const {
    commentLine,
    handleLineDragStart,
    handleLineDragEnter,
    handleLineDragEnd,
    handleLineComment,
    commentForm,
  } = useLineDrag({
    filePath,
    commitSha,
    commentContext,
    InlineCommentForm,
    onExitLineMode: lineMode.onExit,
  })

  const remainingElements = useMemo(
    () =>
      data
        ? buildDiffElements(data.remaining.hunks, data.remaining.newFileLines)
        : [],
    [data],
  )

  const reviewedElements = useMemo(
    () =>
      data
        ? buildDiffElements(data.reviewed.hunks, data.reviewed.newFileLines)
        : [],
    [data],
  )

  const augmentedHunks = useMemo(
    () =>
      singleDiff
        ? augmentHunks(
            singleDiff.hunks,
            fetchedContextLines,
            singleDiff.newFileLines,
          )
        : [],
    [singleDiff, fetchedContextLines],
  )

  const elements = useMemo(
    () =>
      singleDiff
        ? buildDiffElements(augmentedHunks, singleDiff.newFileLines)
        : [],
    [augmentedHunks, singleDiff],
  )

  const handleMarkRegionForSinglePanel = useMemo(
    () => (region: import("@/bindings").HunkId) =>
      handleDualMarkRegion(region, singleSide),
    [handleDualMarkRegion, singleSide],
  )

  const { lineCursor } = useLineMode({
    elements,
    diffViewMode,
    containerRef: diffContainerRef,
    onComment:
      commentContext && InlineCommentForm ? handleLineComment : undefined,
    onMarkRegion: !isSplit ? handleMarkRegionForSinglePanel : undefined,
    ...lineMode,
    state: isSplit ? null : lineMode.state,
  })

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

  if (!data) return null

  if (!hasRemaining && !hasReviewed) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        No content changes
      </div>
    )
  }

  if (isSplit) {
    return (
      <div ref={diffContainerRef}>
        <DualDiff
          remainingElements={remainingElements}
          reviewedElements={reviewedElements}
          lineMode={lineMode}
          onMarkRegion={handleDualMarkRegion}
        />
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
    getInlineThreads,
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
