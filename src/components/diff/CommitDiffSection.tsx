import { useHotkeys } from "react-hotkeys-hook"

import { ErrorDisplay } from "@/components/error"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useCommitFileList } from "@/hooks/useCommitFileList"
import { compareFilePaths } from "@/lib/fileTree"

import { DiffViewToggle } from "./DiffViewToggle"
import {
  FileDiffItem,
  InlineCommentFormProps,
  type PRCommentContext,
} from "./FileDiffItem"
import { useDiffViewMode } from "./useDiffViewMode"

type CommitDiffSectionProps = {
  localDir: string
  commitSha: string
  prComment?: PRCommentContext
  InlineCommentForm?: React.FC<InlineCommentFormProps>
}

export function CommitDiffSection({
  localDir,
  commitSha,
  prComment,
  InlineCommentForm,
}: CommitDiffSectionProps) {
  const { diffViewMode, setDiffViewMode, toggleDiffViewMode } =
    useDiffViewMode()
  const { data, error, isLoading } = useCommitFileList(localDir, commitSha)
  useHotkeys("t", () => toggleDiffViewMode())

  const files =
    data?.files.sort(
      compareFilePaths((file) => (file.newPath || file.oldPath) ?? ""),
    ) ?? []

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

  const reviewedCount = files.filter((f) => f.isReviewed).length
  const progress = files.length > 0 ? (reviewedCount / files.length) * 100 : 0

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium text-muted-foreground">
            Changes ({files.length} file
            {files.length !== 1 ? "s" : ""})
          </h3>
          <div className="flex items-center gap-1.5">
            <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-green-500 transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
            <span className="text-xs text-muted-foreground">
              {reviewedCount}/{files.length}
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
          {files.map((file) => (
            <FileDiffItem
              key={`${data.changeId}-${file.newPath || file.oldPath}`}
              file={file}
              changeId={data.changeId}
              localDir={localDir}
              commitSha={commitSha}
              diffViewMode={diffViewMode}
              prComment={prComment}
              InlineCommentForm={InlineCommentForm}
            />
          ))}
        </div>
      )}
    </div>
  )
}
