import * as Collapsible from "@radix-ui/react-collapsible"
import { useQueryClient } from "@tanstack/react-query"
import { ChevronDown, ChevronRight, Columns2, Copy, Rows3 } from "lucide-react"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import {
  ChangeId,
  commands,
  DiffHunk,
  DiffLine,
  DiffLineType,
  FileChangeStatus,
  FileEntry,
} from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useIsFocused } from "@/hooks/useIsFocused"
import { useFailableQuery, useRpcMutation } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

type DiffViewMode = "unified" | "split"

const DIFF_VIEW_MODE_KEY = "revue-diff-view-mode"

function useDiffViewMode() {
  const [globalMode, _setGlobalMode] = useState<DiffViewMode>(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(DIFF_VIEW_MODE_KEY)
      if (stored === "unified" || stored === "split") {
        return stored
      }
    }
    return "unified"
  })

  const setGlobalMode = (mode: DiffViewMode) => {
    _setGlobalMode(mode)
    localStorage.setItem(DIFF_VIEW_MODE_KEY, globalMode)
  }

  return { globalMode, setGlobalMode }
}

type CommitDiffSectionProps = {
  localDir: string
  commitSha: string
}

export function CommitDiffSection({
  localDir,
  commitSha,
}: CommitDiffSectionProps) {
  const { globalMode, setGlobalMode } = useDiffViewMode()
  const { data, error, isLoading } = useFailableQuery({
    queryKey: ["commit-file-list", localDir, commitSha],
    queryFn: () => commands.getCommitFileList(localDir, commitSha),
  })

  if (isLoading) {
    return (
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-muted-foreground">Changes</h3>
        <p className="text-muted-foreground text-sm">Loading diff...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-muted-foreground">Changes</h3>
        <ErrorDisplay error={error} />
      </div>
    )
  }

  if (!data) {
    return null
  }

  const reviewedCount = data.files.filter((f) => f.isReviewed).length
  const progress =
    data.files.length > 0 ? (reviewedCount / data.files.length) * 100 : 0

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium text-muted-foreground">
            Changes ({data.files.length} file
            {data.files.length !== 1 ? "s" : ""})
          </h3>
          <div className="flex items-center gap-1.5">
            <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-green-500 transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
            <span className="text-xs text-muted-foreground">
              {reviewedCount}/{data.files.length}
            </span>
          </div>
        </div>
        <DiffViewToggle mode={globalMode} onChange={setGlobalMode} />
      </div>
      {data.files.length === 0 ? (
        <Alert>
          <AlertTitle>No Changes</AlertTitle>
          <AlertDescription>
            No file changes found in this commit.
          </AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-3">
          {data.files.map((file) => (
            <FileDiffItem
              key={file.newPath || file.oldPath || ""}
              file={file}
              changeId={data.changeId}
              localDir={localDir}
              commitSha={commitSha}
              globalViewMode={globalMode}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function DiffViewToggle({
  mode,
  onChange,
  size = "default",
}: {
  mode: DiffViewMode
  onChange: (mode: DiffViewMode) => void
  size?: "default" | "small"
}) {
  const iconSize = size === "small" ? "w-3 h-3" : "w-4 h-4"
  const buttonPadding = size === "small" ? "p-1" : "p-1.5"

  return (
    <div
      className="inline-flex items-center rounded-md border bg-muted p-0.5"
      tabIndex={-1}
    >
      <button
        onClick={() => onChange("unified")}
        tabIndex={-1}
        className={cn(
          "inline-flex items-center justify-center rounded-sm transition-colors",
          buttonPadding,
          mode === "unified"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground",
        )}
        title="Unified view"
      >
        <Rows3 className={iconSize} />
      </button>
      <button
        onClick={() => onChange("split")}
        tabIndex={-1}
        className={cn(
          "inline-flex items-center justify-center rounded-sm transition-colors",
          buttonPadding,
          mode === "split"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground",
        )}
        title="Split view"
      >
        <Columns2 className={iconSize} />
      </button>
    </div>
  )
}

function FileDiffItem({
  file,
  changeId,
  localDir,
  commitSha,
  globalViewMode,
}: {
  file: FileEntry
  changeId: ChangeId | null
  localDir: string
  commitSha: string
  globalViewMode: DiffViewMode
}) {
  const [isOpen, setIsOpen] = useState(!file.isReviewed)
  const [localViewMode, setLocalViewMode] = useState<DiffViewMode | null>(null)
  const queryClient = useQueryClient()
  const [ref, isFocused] = useIsFocused()

  // Use local override if set, otherwise use global
  const effectiveViewMode = localViewMode ?? globalViewMode

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
        queryKey: ["commit-file-list", localDir, commitSha],
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
    setTimeout(() => {
      ref.current?.scrollIntoView()
    }, 10)
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

  const handleCopyFilePath = (e: React.MouseEvent) => {
    e.stopPropagation()
    navigator.clipboard.writeText(file.newPath || file.oldPath || "")
  }

  return (
    <Collapsible.Root open={isOpen} onOpenChange={handleOpenChange}>
      <div className="border rounded-lg" tabIndex={1} ref={ref}>
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
            <Collapsible.Trigger asChild>
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
                <span
                  className="font-mono text-sm truncate"
                  title={displayPath}
                >
                  {displayPath}
                </span>
                <Copy onClick={handleCopyFilePath} />
              </div>
            </Collapsible.Trigger>
          </div>
          <div className="flex items-center gap-3 text-xs shrink-0 ml-2">
            <span className="text-green-600 dark:text-green-400">
              +{file.additions}
            </span>
            <span className="text-red-600 dark:text-red-400">
              -{file.deletions}
            </span>
            <DiffViewToggle
              mode={effectiveViewMode}
              onChange={(mode) => setLocalViewMode(mode)}
              size="small"
            />
          </div>
        </div>

        {/* File Content - Lazy loaded */}
        <Collapsible.Content>
          <div className="overflow-x-auto rounded-b-lg">
            {file.isBinary ? (
              <div className="p-4 text-center text-muted-foreground text-sm">
                Binary file changed
              </div>
            ) : (
              <LazyFileDiff
                localDir={localDir}
                commitSha={commitSha}
                filePath={file.newPath || file.oldPath || ""}
                oldPath={
                  file.status === "renamed"
                    ? (file.oldPath ?? undefined)
                    : undefined
                }
                viewMode={effectiveViewMode}
              />
            )}
          </div>
        </Collapsible.Content>
      </div>
    </Collapsible.Root>
  )
}

function LazyFileDiff({
  localDir,
  commitSha,
  filePath,
  oldPath,
  viewMode,
}: {
  localDir: string
  commitSha: string
  filePath: string
  oldPath?: string
  viewMode: DiffViewMode
}) {
  const { data, error, isLoading } = useFailableQuery({
    queryKey: ["file-diff", localDir, commitSha, filePath, oldPath],
    queryFn: () =>
      commands.getFileDiff(localDir, commitSha, filePath, oldPath ?? null),
    staleTime: Infinity,
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

  if (!data) {
    return null
  }

  if (data.length === 0) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        No content changes
      </div>
    )
  }

  if (viewMode === "split") {
    return <SplitDiffView hunks={data} />
  }

  return <UnifiedDiffView hunks={data} />
}

function UnifiedDiffView({ hunks }: { hunks: DiffHunk[] }) {
  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => (
        <div key={idx}>
          {/* Hunk Header */}
          <div className="bg-blue-50 dark:bg-blue-950 px-2 py-1 text-xs font-mono text-blue-700 dark:text-blue-300">
            {hunk.header}
          </div>

          {/* Hunk Lines */}
          <div className="font-mono text-xs">
            {hunk.lines.map((line, lineIdx) => (
              <DiffLineComponent key={lineIdx} line={line} />
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

// Types for split view
type PairedLine = {
  left: DiffLine | null
  right: DiffLine | null
}

// Pair adjacent deletions and additions for side-by-side display
function pairLinesForSplitView(lines: DiffLine[]): PairedLine[] {
  const result: PairedLine[] = []
  let i = 0

  while (i < lines.length) {
    const line = lines[i]

    if (
      line.lineType === "context" ||
      line.lineType === "addeofnl" ||
      line.lineType === "deleofnl"
    ) {
      // Context lines appear on both sides
      result.push({ left: line, right: line })
      i++
    } else if (line.lineType === "deletion") {
      // Collect consecutive deletions
      const deletions: DiffLine[] = []
      while (i < lines.length && lines[i].lineType === "deletion") {
        deletions.push(lines[i])
        i++
      }

      // Collect following consecutive additions
      const additions: DiffLine[] = []
      while (i < lines.length && lines[i].lineType === "addition") {
        additions.push(lines[i])
        i++
      }

      // Pair them up side-by-side
      const maxLen = Math.max(deletions.length, additions.length)
      for (let j = 0; j < maxLen; j++) {
        result.push({
          left: deletions[j] ?? null,
          right: additions[j] ?? null,
        })
      }
    } else if (line.lineType === "addition") {
      // Standalone addition (no preceding deletion)
      result.push({ left: null, right: line })
      i++
    } else {
      i++
    }
  }

  return result
}

function SplitDiffView({ hunks }: { hunks: DiffHunk[] }) {
  return (
    <div className="bg-background">
      {hunks.map((hunk, idx) => {
        const pairedLines = pairLinesForSplitView(hunk.lines)
        return (
          <div key={idx}>
            {/* Hunk Header */}
            <div className="bg-blue-50 dark:bg-blue-950 px-2 py-1 text-xs font-mono text-blue-700 dark:text-blue-300">
              {hunk.header}
            </div>

            {/* Hunk Lines - Split View */}
            <div className="font-mono text-xs">
              {pairedLines.map((pair, lineIdx) => (
                <SplitLineRow key={lineIdx} pair={pair} />
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}

function SplitLineRow({ pair }: { pair: PairedLine }) {
  const leftBg = pair.left
    ? pair.left.lineType === "deletion"
      ? "bg-red-50 dark:bg-red-950/30"
      : "bg-background"
    : "bg-muted/30"

  const rightBg = pair.right
    ? pair.right.lineType === "addition"
      ? "bg-green-50 dark:bg-green-950/30"
      : "bg-background"
    : "bg-muted/30"

  return (
    <div className="flex">
      {/* Left side (old file) */}
      <div className={cn("flex flex-1 min-w-0 border-r border-border", leftBg)}>
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0">
          {pair.left?.oldLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.left
            ? pair.left.tokens.map((token, idx) => (
                <span key={idx} style={{ color: token.color ?? undefined }}>
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>

      {/* Right side (new file) */}
      <div className={cn("flex flex-1 min-w-0", rightBg)}>
        <span className="w-10 text-right pr-2 text-muted-foreground select-none shrink-0">
          {pair.right?.newLineno ?? ""}
        </span>
        <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word overflow-hidden">
          {pair.right
            ? pair.right.tokens.map((token, idx) => (
                <span key={idx} style={{ color: token.color ?? undefined }}>
                  {token.content}
                </span>
              ))
            : null}
        </span>
      </div>
    </div>
  )
}

function DiffLineComponent({ line }: { line: DiffLine }) {
  const { bgColor } = getLineStyle(line.lineType)

  return (
    <div className={cn("flex hover:bg-muted/30", bgColor)}>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0">
        {line.oldLineno || ""}
      </span>
      <span className="w-12 text-right pr-2 text-muted-foreground select-none shrink-0">
        {line.newLineno || ""}
      </span>
      <span className="flex-1 pl-2 whitespace-pre-wrap wrap-break-word">
        {line.tokens.map((token, idx) => (
          <span key={idx} style={{ color: token.color ?? undefined }}>
            {token.content}
          </span>
        ))}
      </span>
    </div>
  )
}

function getStatusStyle(status: FileChangeStatus): {
  bgColor: string
  textColor: string
  label: string
} {
  switch (status) {
    case "added":
      return {
        bgColor: "bg-green-100 dark:bg-green-900",
        textColor: "text-green-800 dark:text-green-200",
        label: "Added",
      }
    case "modified":
      return {
        bgColor: "bg-blue-100 dark:bg-blue-900",
        textColor: "text-blue-800 dark:text-blue-200",
        label: "Modified",
      }
    case "deleted":
      return {
        bgColor: "bg-red-100 dark:bg-red-900",
        textColor: "text-red-800 dark:text-red-200",
        label: "Deleted",
      }
    case "renamed":
      return {
        bgColor: "bg-purple-100 dark:bg-purple-900",
        textColor: "text-purple-800 dark:text-purple-200",
        label: "Renamed",
      }
    case "copied":
      return {
        bgColor: "bg-yellow-100 dark:bg-yellow-900",
        textColor: "text-yellow-800 dark:text-yellow-200",
        label: "Copied",
      }
    case "typechange":
      return {
        bgColor: "bg-orange-100 dark:bg-orange-900",
        textColor: "text-orange-800 dark:text-orange-200",
        label: "Type",
      }
    default:
      return {
        bgColor: "bg-gray-100 dark:bg-gray-900",
        textColor: "text-gray-800 dark:text-gray-200",
        label: "Changed",
      }
  }
}

function getLineStyle(lineType: DiffLineType): {
  bgColor: string
} {
  switch (lineType) {
    case "addition":
      return {
        bgColor: "bg-green-50 dark:bg-green-950/30",
      }
    case "deletion":
      return {
        bgColor: "bg-red-50 dark:bg-red-950/30",
      }
    case "context":
    case "addeofnl":
    case "deleofnl":
    default:
      return {
        bgColor: "bg-background",
      }
  }
}
