import { useState } from "react"
import * as Collapsible from "@radix-ui/react-collapsible"
import { ChevronRight, ChevronDown } from "lucide-react"
import { useQueryClient } from "@tanstack/react-query"
import {
  commands,
  FileEntry,
  DiffHunk,
  DiffLine,
  FileChangeStatus,
  DiffLineType,
  ChangeId,
} from "@/bindings"
import { useFailableQuery, useRpcMutation } from "@/hooks/useRpcQuery"
import { ErrorDisplay } from "@/components/error"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { cn } from "@/lib/utils"

type CommitDiffSectionProps = {
  localDir: string
  commitSha: string
}

export function CommitDiffSection({
  localDir,
  commitSha,
}: CommitDiffSectionProps) {
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

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-muted-foreground">
        Changes ({data.files.length} file{data.files.length !== 1 ? "s" : ""})
      </h3>
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
            />
          ))}
        </div>
      )}
    </div>
  )
}

function FileDiffItem({
  file,
  changeId,
  localDir,
  commitSha,
}: {
  file: FileEntry
  changeId: ChangeId | null
  localDir: string
  commitSha: string
}) {
  const [isOpen, setIsOpen] = useState(!file.isReviewed)
  const queryClient = useQueryClient()

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
        queryKey: ["commit-file-list", localDir],
      })
    },
  })

  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!file.patchId || !changeId) return
    const isReviewd = e.target.checked
    toggleMutation.mutate(isReviewd)
    if (isReviewd) {
      setIsOpen(false)
    }
  }

  const displayPath = file.newPath || file.oldPath || "unknown"
  const { bgColor, textColor, label } = getStatusStyle(file.status)
  // Can only review if we have both patchId and changeId
  const canBeReviewed =
    file.patchId !== null && file.patchId !== undefined && changeId !== null

  return (
    <Collapsible.Root open={isOpen} onOpenChange={setIsOpen}>
      <div className="border rounded-lg">
        {/* Sticky File Header */}
        <div className="sticky top-0 z-20 flex items-center justify-between p-3 bg-muted rounded-t-lg border-b">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            {/* Checkbox for reviewed status */}
            {canBeReviewed && (
              <input
                type="checkbox"
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
                <div className="flex-shrink-0">
                  {isOpen ? (
                    <ChevronDown className="w-4 h-4" />
                  ) : (
                    <ChevronRight className="w-4 h-4" />
                  )}
                </div>
                <span
                  className={cn(
                    "text-xs font-semibold uppercase px-2 py-1 rounded flex-shrink-0",
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
              </div>
            </Collapsible.Trigger>
          </div>
          <div className="flex items-center gap-3 text-xs flex-shrink-0 ml-2">
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
          <div className="overflow-hidden rounded-b-lg">
            {file.isBinary ? (
              <div className="p-4 text-center text-muted-foreground text-sm">
                Binary file changed
              </div>
            ) : (
              <LazyFileDiff
                localDir={localDir}
                commitSha={commitSha}
                filePath={file.newPath || file.oldPath || ""}
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
}: {
  localDir: string
  commitSha: string
  filePath: string
}) {
  const { data, error, isLoading } = useFailableQuery({
    queryKey: ["file-diff", localDir, commitSha, filePath],
    queryFn: () => commands.getFileDiff(localDir, commitSha, filePath),
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

  return <UnifiedDiffView hunks={data.hunks} />
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
      <span className="flex-1 pl-2 whitespace-pre">
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
