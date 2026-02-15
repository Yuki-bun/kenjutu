import { ChevronDown, ChevronUp, UnfoldVertical } from "lucide-react"

import { type HunkGap } from "./hunkGaps"
import { ExpandDirection } from "./SplitDiff"

export function GapRow({
  gap,
  isLast,
  onExpandGap,
}: {
  gap: HunkGap
  isLast: boolean
  onExpandGap: (gap: HunkGap, direction: ExpandDirection) => void
}) {
  return (
    <GapIndicator
      hiddenLineCount={gap.count}
      showExpandUp={!isLast}
      showExpandDown={gap.newStart !== 1}
      onExpand={(dir) => onExpandGap(gap, dir)}
    />
  )
}

type GapIndicatorProps = {
  hiddenLineCount: number
  showExpandUp: boolean
  showExpandDown: boolean
  onExpand: (direction: ExpandDirection) => void
}

function GapIndicator({
  hiddenLineCount,
  showExpandUp,
  showExpandDown,
  onExpand,
}: GapIndicatorProps) {
  if (hiddenLineCount === 0) return null

  const showAll = showExpandUp && showExpandDown

  return (
    <div className="flex items-center bg-blue-50/50 dark:bg-blue-950/20 border-y border-border text-xs text-blue-700 dark:text-blue-300">
      <div className="flex shrink-0 px-1 gap-0.5">
        {showExpandDown && (
          <ExpandButton direction="down" onClick={() => onExpand("down")} />
        )}
        {showExpandUp && (
          <ExpandButton direction="up" onClick={() => onExpand("up")} />
        )}
        {showAll && (
          <button
            type="button"
            className="p-0.5 rounded hover:bg-blue-200 dark:hover:bg-blue-800 transition-colors disabled:opacity-50"
            onClick={(e) => {
              e.stopPropagation()
              onExpand("all")
            }}
            title="Expand all"
          >
            <UnfoldVertical className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
      <span className="text-muted-foreground text-xs py-0.5">
        {hiddenLineCount} hidden lines
      </span>
    </div>
  )
}

type ExpandButtonProps = {
  direction: ExpandDirection
  onClick: () => void
}

function ExpandButton({ direction, onClick }: ExpandButtonProps) {
  const Icon = direction === "up" ? ChevronUp : ChevronDown
  return (
    <button
      type="button"
      className="p-0.5 rounded hover:bg-blue-200 dark:hover:bg-blue-800 transition-colors disabled:opacity-50"
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      title={`Expand ${direction}`}
    >
      <Icon className="w-3.5 h-3.5" />
    </button>
  )
}
