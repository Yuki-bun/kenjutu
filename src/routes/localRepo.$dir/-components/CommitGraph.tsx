import { useMemo } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import type {
  CommitGraph as CommitGraphData,
  CommitRow,
  GraphEdge,
  GraphRow,
  JjCommit,
} from "@/bindings"
import { Pane, PANEL_KEYS, usePaneItem } from "@/components/Pane"
import { useCommitFileList } from "@/hooks/useCommitFileList"
import { cn } from "@/lib/utils"

type CommitGraphProps = {
  localDir: string
  graph: CommitGraphData
  selectedChangeId: string | null
  onSelectCommit: (commit: JjCommit) => void
}

const COL_WIDTH = 16
const ROW_HEIGHT = 32

function colX(col: number) {
  return COL_WIDTH * col + COL_WIDTH / 2
}

function rowY(row: number) {
  return ROW_HEIGHT * row + ROW_HEIGHT / 2
}

function edgePath(fromRow: number, edge: GraphEdge): string {
  const x1 = colX(edge.fromColumn)
  const y1 = rowY(fromRow)
  const x2 = colX(edge.toColumn)
  const y2 = rowY(edge.toRow)

  if (edge.fromColumn === edge.toColumn) {
    return `M ${x1} ${y1} L ${x2} ${y2}`
  }

  // Cross-column: go straight down, then curve into the target column
  const bendY = y2 - ROW_HEIGHT * 0.4
  return `M ${x1} ${y1} L ${x1} ${bendY} Q ${x1} ${y2} ${x2} ${y2}`
}

function CommitGraphCommitRow({
  localDir,
  commitRow,
  svgWidth,
  isSelected,
  onClick,
}: {
  localDir: string
  commitRow: CommitRow
  svgWidth: number
  isSelected: boolean
  onClick: () => void
}) {
  const { commit } = commitRow
  const { ref, isFocused } = usePaneItem<HTMLButtonElement>(commit.changeId)

  const { data } = useCommitFileList(localDir, commit.commitId)

  useHotkeys("c", () => navigator.clipboard.writeText(commit.changeId), {
    enabled: isFocused,
  })

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
      style={{ height: ROW_HEIGHT }}
      className={cn(
        "w-full flex items-center gap-2 px-2 text-left hover:bg-accent rounded transition-colors focusKey",
        isSelected && "bg-accent",
        commit.isImmutable && "opacity-60",
      )}
    >
      <div style={{ width: svgWidth }} className="shrink-0" />

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
        <span className="ml-1 truncate text-xs">
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

function ElisionGraphRow({ svgWidth }: { svgWidth: number }) {
  return (
    <div
      style={{ height: ROW_HEIGHT }}
      className="w-full flex items-center gap-2 px-2 opacity-50"
    >
      <div style={{ width: svgWidth }} className="shrink-0" />
      <span className="text-xs text-muted-foreground italic">
        ~ (elided revisions)
      </span>
    </div>
  )
}

function isCommitRow(row: GraphRow): row is GraphRow & { type: "commit" } {
  return row.type === "commit"
}

export function CommitGraph({
  localDir,
  graph,
  selectedChangeId,
  onSelectCommit,
}: CommitGraphProps) {
  const svgWidth = graph.maxColumns * COL_WIDTH
  const svgHeight = graph.rows.length * ROW_HEIGHT

  // Collect all edges and passing-column segments for SVG rendering
  const { edges, passingSegments, nodes, elisionNodes } = useMemo(() => {
    const edges: { fromRow: number; edge: GraphEdge }[] = []
    const passingSegments: { col: number; row: number }[] = []
    const nodes: { row: GraphRow & { type: "commit" }; idx: number }[] = []
    const elisionNodes: { row: GraphRow & { type: "elision" }; idx: number }[] =
      []

    for (const row of graph.rows) {
      if (isCommitRow(row)) {
        nodes.push({ row, idx: row.row })
        for (const edge of row.edges) {
          edges.push({ fromRow: row.row, edge })
        }
        for (const col of row.passingColumns) {
          passingSegments.push({ col, row: row.row })
        }
      } else {
        elisionNodes.push({
          row: row as GraphRow & { type: "elision" },
          idx: row.row,
        })
        for (const col of row.passingColumns) {
          passingSegments.push({ col, row: row.row })
        }
      }
    }

    return { edges, passingSegments, nodes, elisionNodes }
  }, [graph])

  return (
    <Pane
      className="font-mono text-sm relative"
      panelKey={PANEL_KEYS.commitGraph}
    >
      <svg
        className="absolute top-0 left-0 pointer-events-none"
        width={svgWidth}
        height={svgHeight}
        style={{ marginLeft: 8 }}
      >
        {/* Pass-through lines: short vertical segments for branches passing through */}
        {passingSegments.map((seg, i) => (
          <line
            key={`pass-${i}`}
            x1={colX(seg.col)}
            y1={rowY(seg.row) - ROW_HEIGHT / 2}
            x2={colX(seg.col)}
            y2={rowY(seg.row) + ROW_HEIGHT / 2}
            stroke="var(--color-muted-foreground)"
            opacity={0.25}
            strokeWidth={2}
          />
        ))}

        {/* Edges from commits to their parents */}
        {edges.map((e, i) => (
          <path
            key={`edge-${i}`}
            d={edgePath(e.fromRow, e.edge)}
            stroke="var(--color-muted-foreground)"
            opacity={0.4}
            strokeWidth={2}
            fill="none"
            strokeDasharray={e.edge.edgeType === "elided" ? "4 3" : undefined}
          />
        ))}

        {/* Commit node circles */}
        {nodes.map((n) => (
          <circle
            key={n.row.commit.changeId}
            cx={colX(n.row.column)}
            cy={rowY(n.idx)}
            r={n.row.commit.isWorkingCopy ? 5 : 4}
            fill={
              n.row.commit.isWorkingCopy
                ? "var(--color-green-500)"
                : "var(--color-blue-500)"
            }
          />
        ))}

        {/* Elision markers: tilde character */}
        {elisionNodes.map((n) => (
          <text
            key={`elision-${n.idx}`}
            x={colX(n.row.column)}
            y={rowY(n.idx) + 4}
            textAnchor="middle"
            fill="var(--color-muted-foreground)"
            fontSize={14}
            fontWeight="bold"
          >
            ~
          </text>
        ))}
      </svg>

      {/* Row content */}
      {graph.rows.map((row) =>
        isCommitRow(row) ? (
          <CommitGraphCommitRow
            key={row.commit.changeId}
            localDir={localDir}
            commitRow={row}
            svgWidth={svgWidth}
            isSelected={row.commit.changeId === selectedChangeId}
            onClick={() => onSelectCommit(row.commit)}
          />
        ) : (
          <ElisionGraphRow key={`elision-${row.row}`} svgWidth={svgWidth} />
        ),
      )}
    </Pane>
  )
}
