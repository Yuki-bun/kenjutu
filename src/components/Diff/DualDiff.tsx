import { DiffElement } from "./hunkGaps"
import { UnifiedHunkLines } from "./UnifiedDiff"

type DualDiffProps = {
  remainingElements: DiffElement[]
  reviewedElements: DiffElement[]
}

export function DualDiff({
  remainingElements,
  reviewedElements,
}: DualDiffProps) {
  return (
    <div className="grid grid-cols-2 divide-x">
      <DualPanel label="Remaining" elements={remainingElements} />
      <DualPanel label="Reviewed" elements={reviewedElements} />
    </div>
  )
}

function DualPanel({
  label,
  elements,
}: {
  label: string
  elements: DiffElement[]
}) {
  const hunkElements = elements.filter((el) => el.type === "hunk")

  return (
    <div className="bg-background">
      <div className="px-3 py-1 text-xs font-medium text-muted-foreground bg-muted/50 border-b">
        {label}
      </div>
      {hunkElements.length === 0 ? (
        <div className="p-4 text-center text-muted-foreground text-sm">
          No changes
        </div>
      ) : (
        hunkElements.map((el, idx) => (
          <UnifiedHunkLines
            key={`hunk-${idx}`}
            hunk={el.hunk}
            elementIndex={idx}
          />
        ))
      )}
    </div>
  )
}
