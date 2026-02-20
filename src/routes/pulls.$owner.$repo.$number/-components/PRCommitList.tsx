import { PRCommit } from "@/bindings"
import { Pane, PANEL_KEYS, usePaneItem } from "@/components/Pane"
import { useCommitFileList } from "@/hooks/useCommitFileList"
import { cn } from "@/lib/utils"

const ROW_HEIGHT = 32

type PRCommitListProps = {
  localDir: string
  commits: PRCommit[]
  selectedCommitSha: string | null
  onSelectCommit: (commit: PRCommit) => void
}

export function PRCommitList({
  localDir,
  commits,
  selectedCommitSha,
  onSelectCommit,
}: PRCommitListProps) {
  return (
    <div className="px-2 py-3">
      <h3 className="text-xs font-medium text-muted-foreground mb-2">
        Commits ({commits.length})
      </h3>
      <Pane className="font-mono text-sm" panelKey={PANEL_KEYS.prCommitList}>
        {commits.map((commit) => (
          <PRCommitRow
            key={commit.sha}
            localDir={localDir}
            commit={commit}
            isSelected={commit.sha === selectedCommitSha}
            onClick={() => onSelectCommit(commit)}
          />
        ))}
      </Pane>
    </div>
  )
}

function PRCommitRow({
  localDir,
  commit,
  isSelected,
  onClick,
}: {
  localDir: string
  commit: PRCommit
  isSelected: boolean
  onClick: () => void
}) {
  const { ref } = usePaneItem<HTMLButtonElement>(commit.sha)

  const { data } = useCommitFileList(localDir, commit.sha)

  const progress = data
    ? {
        reviewed: data.files.filter((f) => f.reviewStatus === "reviewed")
          .length,
        total: data.files.filter((f) => f.reviewStatus !== "reviewedReverted")
          .length,
      }
    : null

  return (
    <button
      ref={ref}
      onClick={onClick}
      onFocus={onClick}
      style={{ height: ROW_HEIGHT }}
      className={cn(
        "w-full flex items-center gap-2 px-2 text-left hover:bg-accent rounded transition-colors focusKey",
        isSelected && "bg-accent",
      )}
    >
      <span className="flex-1 min-w-0 flex items-center gap-1">
        <span className="font-mono text-xs px-1 rounded bg-muted text-muted-foreground shrink-0">
          {commit.sha.slice(0, 8)}
        </span>
        {progress && progress.total > 0 && (
          <span
            className="shrink-0"
            title={`${progress.reviewed}/${progress.total} files reviewed`}
          >
            {progress.reviewed === progress.total ? (
              <span className="text-green-500 text-xs">&#10003;</span>
            ) : (
              <span className="inline-flex w-8 h-1.5 bg-muted rounded-full overflow-hidden">
                <span
                  className="h-full bg-green-500 transition-all duration-300"
                  style={{
                    width: `${(progress.reviewed / progress.total) * 100}%`,
                  }}
                />
              </span>
            )}
          </span>
        )}
        <span className="ml-1 truncate">
          {commit.summary || (
            <span className="italic text-muted-foreground">
              (no description)
            </span>
          )}
        </span>
      </span>
    </button>
  )
}
