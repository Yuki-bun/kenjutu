import { JjCommit } from "@/bindings"
import { cn } from "@/lib/utils"

type CommitGraphProps = {
  commits: JjCommit[]
  selectedChangeId: string | null
  onSelectCommit: (commit: JjCommit) => void
}

// Graph building types
type GraphRow = {
  commit: JjCommit
  columns: GraphColumn[]
  nodeColumn: number
}

type GraphColumn =
  | { type: "empty" }
  | { type: "line" }
  | { type: "node"; isWorkingCopy: boolean }
  | { type: "merge-left" }
  | { type: "merge-right" }
  | { type: "branch-out" }

function buildGraph(commits: JjCommit[]): { rows: GraphRow[] } {
  // Build a map of change_id -> commit for quick lookup
  const commitMap = new Map<string, JjCommit>()
  for (const commit of commits) {
    commitMap.set(commit.changeId, commit)
  }

  // Track which columns are "active" (have a line going through them)
  // Each active column tracks which change_id it's waiting for
  const activeColumns: (string | null)[] = []
  const rows: GraphRow[] = []

  for (const commit of commits) {
    // Find if this commit has a reserved column (parent of a previous commit)
    let nodeColumn = activeColumns.indexOf(commit.changeId)

    if (nodeColumn === -1) {
      // No reserved column, find first empty slot or append
      nodeColumn = activeColumns.indexOf(null)
      if (nodeColumn === -1) {
        nodeColumn = activeColumns.length
        activeColumns.push(null)
      }
    }

    // Build the column state for this row
    const columns: GraphColumn[] = []

    for (let i = 0; i < Math.max(activeColumns.length, nodeColumn + 1); i++) {
      if (i === nodeColumn) {
        columns.push({ type: "node", isWorkingCopy: commit.isWorkingCopy })
      } else if (activeColumns[i] !== null && activeColumns[i] !== undefined) {
        columns.push({ type: "line" })
      } else {
        columns.push({ type: "empty" })
      }
    }

    rows.push({
      commit,
      columns,
      nodeColumn,
    })

    // Update active columns for parents
    // Clear the current column first
    activeColumns[nodeColumn] = null

    // Reserve columns for parents
    for (let i = 0; i < commit.parents.length; i++) {
      const parentId = commit.parents[i]
      // Check if parent is in our commit list
      if (!commitMap.has(parentId)) continue

      // Check if parent already has a reserved column
      const existingCol = activeColumns.indexOf(parentId)
      if (existingCol !== -1) continue

      // Find a column for this parent
      if (i === 0) {
        // First parent takes the node's column
        activeColumns[nodeColumn] = parentId
      } else {
        // Additional parents get new columns
        const emptyCol = activeColumns.indexOf(null)
        if (emptyCol !== -1) {
          activeColumns[emptyCol] = parentId
        } else {
          activeColumns.push(parentId)
        }
      }
    }

    // Trim trailing nulls
    while (
      activeColumns.length > 0 &&
      activeColumns[activeColumns.length - 1] === null
    ) {
      activeColumns.pop()
    }
  }

  return { rows }
}

function GraphColumnChar({ column }: { column: GraphColumn }) {
  switch (column.type) {
    case "empty":
      return <span className="text-muted-foreground"> </span>
    case "line":
      return <span className="text-muted-foreground">|</span>
    case "node":
      return column.isWorkingCopy ? (
        <span className="text-green-500 font-bold">@</span>
      ) : (
        <span className="text-blue-500">o</span>
      )
    case "merge-left":
      return <span className="text-muted-foreground">|</span>
    case "merge-right":
      return <span className="text-muted-foreground">|</span>
    case "branch-out":
      return <span className="text-muted-foreground">|</span>
    default:
      return <span> </span>
  }
}

function CommitGraphRow({
  row,
  isSelected,
  onClick,
}: {
  row: GraphRow
  isSelected: boolean
  onClick: () => void
}) {
  const { commit, columns } = row

  return (
    <button
      onClick={onClick}
      className={cn(
        "w-full flex items-start gap-2 px-2 py-1 text-left hover:bg-accent rounded transition-colors",
        isSelected && "bg-accent",
        commit.isImmutable && "opacity-60",
      )}
    >
      {/* Graph visualization */}
      <span className="font-mono shrink-0 flex">
        {columns.map((col, idx) => (
          <div key={idx} className="w-3">
            <GraphColumnChar column={col} />
          </div>
        ))}
      </span>

      {/* Commit info */}
      <span className="flex-1 min-w-0">
        <span
          className={cn(
            "font-mono text-xs px-1 rounded",
            commit.isWorkingCopy
              ? "bg-green-500/20 text-green-700 dark:text-green-300"
              : "bg-muted text-muted-foreground",
          )}
        >
          {commit.changeId.slice(0, 8)}
        </span>
        <span className="ml-2 w-full">
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

export function CommitGraph({
  commits,
  selectedChangeId,
  onSelectCommit,
}: CommitGraphProps) {
  const graph = buildGraph(commits)

  return (
    <div className="font-mono text-sm">
      {graph.rows.map((row) => (
        <CommitGraphRow
          key={row.commit.changeId}
          row={row}
          isSelected={row.commit.changeId === selectedChangeId}
          onClick={() => onSelectCommit(row.commit)}
        />
      ))}
    </div>
  )
}
