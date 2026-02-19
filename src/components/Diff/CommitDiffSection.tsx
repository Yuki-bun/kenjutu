import { createContext, useContext, useMemo } from "react"

import { ChangeId, FileEntry } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { useCommitFileList } from "@/hooks/useCommitFileList"
import { compareFilePaths } from "@/lib/fileTree"

import { DiffViewMode, useDiffViewMode } from "./useDiffViewMode"

type DiffContextValue = {
  files: FileEntry[]
  localDir: string
  commitSha: string
  changeId: ChangeId
  diffViewMode: DiffViewMode
  setDiffViewMode: (mode: DiffViewMode) => void
  toggleDiffViewMode: () => void
}

const DiffContext = createContext<DiffContextValue | null>(null)

export function useDiffContext(): DiffContextValue {
  const ctx = useContext(DiffContext)
  if (!ctx) {
    throw new Error("useDiffContext must be used within <CommitDiffSection>")
  }
  return ctx
}

type CommitDiffSectionProps = {
  localDir: string
  commitSha: string
  children: React.ReactNode
}

export function CommitDiffSection({
  localDir,
  commitSha,
  children,
}: CommitDiffSectionProps) {
  const { diffViewMode, setDiffViewMode, toggleDiffViewMode } =
    useDiffViewMode()
  const { data, error, isLoading } = useCommitFileList(localDir, commitSha)

  const files = useMemo(
    () =>
      data?.files.sort(
        compareFilePaths((file) => (file.newPath || file.oldPath) ?? ""),
      ) ?? [],
    [data?.files],
  )

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
    <DiffContext.Provider
      value={{
        files,
        localDir,
        commitSha,
        changeId: data.changeId,
        diffViewMode,
        setDiffViewMode,
        toggleDiffViewMode,
      }}
    >
      {children}
    </DiffContext.Provider>
  )
}
