import * as Collapsible from "@radix-ui/react-collapsible"
import { useQueryClient } from "@tanstack/react-query"
import { ChevronDown, ChevronRight, Copy } from "lucide-react"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { ChangeId, commands, FileEntry } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { useScrollFocusItem } from "@/context/ScrollFocusContext"
import { useFailableQuery, useRpcMutation } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

import { getStatusStyle } from "./diffStyles"
import { SplitDiffView, UnifiedDiffView } from "./DiffViews"
import { DiffViewMode } from "./useDiffViewMode"

export function FileDiffItem({
  file,
  changeId,
  localDir,
  commitSha,
  diffViewMode,
}: {
  file: FileEntry
  changeId: ChangeId | null
  localDir: string
  commitSha: string
  diffViewMode: DiffViewMode
}) {
  const [isOpen, setIsOpen] = useState(!file.isReviewed)
  const queryClient = useQueryClient()
  const filePath = file.newPath || file.oldPath || ""

  const { ref, isFocused } = useScrollFocusItem<HTMLDivElement>(filePath)

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
      <div className="border rounded-lg" tabIndex={0} ref={ref}>
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
                diffViewMode={diffViewMode}
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
  diffViewMode,
}: {
  localDir: string
  commitSha: string
  filePath: string
  oldPath?: string
  diffViewMode: DiffViewMode
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

  if (diffViewMode === "split") {
    return <SplitDiffView hunks={data} />
  }

  return <UnifiedDiffView hunks={data} />
}
