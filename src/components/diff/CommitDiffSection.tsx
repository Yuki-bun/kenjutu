import { useHotkeys } from "react-hotkeys-hook"

import { commands } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

import { DiffViewToggle } from "./DiffViewToggle"
import { FileDiffItem } from "./FileDiffItem"
import { useDiffViewMode } from "./useDiffViewMode"

type CommitDiffSectionProps = {
  localDir: string
  commitSha: string
}

export function CommitDiffSection({
  localDir,
  commitSha,
}: CommitDiffSectionProps) {
  const { diffViewMode, setDiffViewMode, toggleDiffViewMode } =
    useDiffViewMode()
  const { data, error, isLoading } = useFailableQuery({
    queryKey: queryKeys.commitFileList(localDir, commitSha),
    queryFn: () => commands.getCommitFileList(localDir, commitSha),
  })
  useHotkeys("t", () => toggleDiffViewMode())

  const files =
    data?.files.sort((a, b) => {
      const pathA = a.newPath || a.oldPath || ""
      const pathB = b.newPath || b.oldPath || ""
      return pathA.localeCompare(pathB)
    }) ?? []

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
        <DiffViewToggle mode={diffViewMode} setMode={setDiffViewMode} />
      </div>
      {files.length === 0 ? (
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
              key={`${data.changeId}-${file.newPath || file.oldPath}`}
              file={file}
              changeId={data.changeId}
              localDir={localDir}
              commitSha={commitSha}
              diffViewMode={diffViewMode}
            />
          ))}
        </div>
      )}
    </div>
  )
}
