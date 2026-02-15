import { MessageSquarePlus } from "lucide-react"

import { cn } from "@/lib/utils"

export function LineNumberGutter({
  lineNumber,
  side,
  className,
  onLineDragStart,
  onLineDragEnter,
  onLineDragEnd,
  children,
}: {
  lineNumber: number | null
  side: "LEFT" | "RIGHT"
  className?: string
  onLineDragStart?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnter?: (line: number, side: "LEFT" | "RIGHT") => void
  onLineDragEnd?: () => void
  children: React.ReactNode
}) {
  const interactive = lineNumber != null

  return (
    <span
      className={cn(
        "text-right pr-2 text-muted-foreground select-none shrink-0 relative cursor-pointer",
        className,
      )}
      onMouseDown={
        interactive
          ? (e) => {
              if (onLineDragStart) {
                e.preventDefault()
                onLineDragStart(lineNumber, side)
              }
            }
          : undefined
      }
      onMouseEnter={
        interactive ? () => onLineDragEnter?.(lineNumber, side) : undefined
      }
      onMouseUp={() => onLineDragEnd?.()}
    >
      {onLineDragStart && interactive && (
        <span className="absolute left-0 top-1/2 -translate-y-1/2 opacity-0 group-hover/line:opacity-100 transition-opacity bg-blue-500 text-white rounded-sm p-0.5 hover:bg-blue-600 z-10">
          <MessageSquarePlus className="w-3 h-3" />
        </span>
      )}
      {children}
    </span>
  )
}
