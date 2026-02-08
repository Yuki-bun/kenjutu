import { commands, JjCommit } from "@/bindings"
import { ScrollFocus, useScrollFocusItem } from "@/components/ScrollFocus"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

type CommitGraphProps = {
  localDir: string
  commits: JjCommit[]
  selectedChangeId: string | null
  onSelectCommit: (commit: JjCommit) => void
}

const COL_WIDTH = 16
const ROW_HEIGHT = 32

type GraphNode = {
  commit: JjCommit
  column: number
  row: number
}

type GraphEdge = {
  fromRow: number
  fromCol: number
  toRow: number
  toCol: number
}

type GraphLayout = {
  nodes: GraphNode[]
  edges: GraphEdge[]
  maxColumns: number
}

function buildGraph(commits: JjCommit[]): GraphLayout {
  const commitMap = new Map<string, JjCommit>()
  for (const commit of commits) {
    commitMap.set(commit.changeId, commit)
  }

  const activeColumns: (string | null)[] = []
  const nodes: GraphNode[] = []
  const nodeIndex = new Map<string, { row: number; column: number }>()

  for (let row = 0; row < commits.length; row++) {
    const commit = commits[row]

    let nodeColumn = activeColumns.indexOf(commit.changeId)

    if (nodeColumn === -1) {
      nodeColumn = activeColumns.indexOf(null)
      if (nodeColumn === -1) {
        nodeColumn = activeColumns.length
        activeColumns.push(null)
      }
    }

    nodes.push({ commit, column: nodeColumn, row })
    nodeIndex.set(commit.changeId, { row, column: nodeColumn })

    // Update active columns for parents
    activeColumns[nodeColumn] = null

    for (let i = 0; i < commit.parents.length; i++) {
      const parentId = commit.parents[i]
      if (!commitMap.has(parentId)) continue

      const existingCol = activeColumns.indexOf(parentId)
      if (existingCol !== -1) continue

      if (i === 0) {
        activeColumns[nodeColumn] = parentId
      } else {
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

  // Build edges from child to parent
  const edges: GraphEdge[] = []
  for (const node of nodes) {
    for (const parentId of node.commit.parents) {
      const parent = nodeIndex.get(parentId)
      if (!parent) continue
      edges.push({
        fromRow: node.row,
        fromCol: node.column,
        toRow: parent.row,
        toCol: parent.column,
      })
    }
  }

  let maxColumns = 0
  for (const node of nodes) {
    if (node.column + 1 > maxColumns) maxColumns = node.column + 1
  }

  return { nodes, edges, maxColumns }
}

function colX(col: number) {
  return COL_WIDTH * col + COL_WIDTH / 2
}

function rowY(row: number) {
  return ROW_HEIGHT * row + ROW_HEIGHT / 2
}

function edgePath(edge: GraphEdge): string {
  const x1 = colX(edge.fromCol)
  const y1 = rowY(edge.fromRow)
  const x2 = colX(edge.toCol)
  const y2 = rowY(edge.toRow)

  if (edge.fromCol === edge.toCol) {
    // Straight vertical line
    return `M ${x1} ${y1} L ${x2} ${y2}`
  }

  // Cross-column: go straight down, then curve into the parent's column
  // The bend happens in the last row-gap before the parent
  const bendY = y2 - ROW_HEIGHT * 0.4
  return `M ${x1} ${y1} L ${x1} ${bendY} Q ${x1} ${y2} ${x2} ${y2}`
}

function CommitGraphRow({
  localDir,
  node,
  svgWidth,
  isSelected,
  onClick,
}: {
  localDir: string
  node: GraphNode
  svgWidth: number
  isSelected: boolean
  onClick: () => void
}) {
  const { commit } = node
  const { ref } = useScrollFocusItem<HTMLButtonElement>(node.commit.changeId)

  const { data } = useFailableQuery({
    queryKey: ["commit-file-list", localDir, commit.commitId],
    queryFn: () => commands.getCommitFileList(localDir, commit.commitId),
  })

  const progress = data
    ? {
        reviewed: data.files.filter((f) => f.isReviewed).length,
        total: data.files.length,
      }
    : null

  return (
    <button
      ref={ref}
      onClick={onClick}
      onFocus={onClick}
      style={{ height: ROW_HEIGHT }}
      className={cn(
        "w-full flex items-center gap-2 px-2 text-left hover:bg-accent rounded transition-colors",
        isSelected && "bg-accent",
        commit.isImmutable && "opacity-60",
      )}
    >
      {/* Spacer to push content past the SVG area */}
      <div style={{ width: svgWidth }} className="shrink-0" />

      {/* Commit info */}
      <span className="flex-1 min-w-0 flex items-center gap-1">
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

export function CommitGraph({
  localDir,
  commits,
  selectedChangeId,
  onSelectCommit,
}: CommitGraphProps) {
  const graph = buildGraph(commits)
  const svgWidth = graph.maxColumns * COL_WIDTH
  const svgHeight = commits.length * ROW_HEIGHT

  return (
    <ScrollFocus
      className="font-mono text-sm relative"
      panelKey={"commit-graph"}
    >
      <svg
        className="absolute top-0 left-0 pointer-events-none"
        width={svgWidth}
        height={svgHeight}
        style={{ marginLeft: 8 }}
      >
        {graph.edges.map((edge, i) => (
          <path
            key={i}
            d={edgePath(edge)}
            stroke="var(--color-muted-foreground)"
            opacity={0.4}
            strokeWidth={2}
            fill="none"
          />
        ))}
        {graph.nodes.map((node) => (
          <circle
            key={node.commit.changeId}
            cx={colX(node.column)}
            cy={rowY(node.row)}
            r={node.commit.isWorkingCopy ? 5 : 4}
            fill={
              node.commit.isWorkingCopy
                ? "var(--color-green-500)"
                : "var(--color-blue-500)"
            }
          />
        ))}
      </svg>
      {graph.nodes.map((node) => (
        <CommitGraphRow
          key={node.commit.changeId}
          localDir={localDir}
          node={node}
          svgWidth={svgWidth}
          isSelected={node.commit.changeId === selectedChangeId}
          onClick={() => onSelectCommit(node.commit)}
        />
      ))}
    </ScrollFocus>
  )
}
