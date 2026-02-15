import { Columns2, Rows3 } from "lucide-react"

import { cn } from "@/lib/utils"

import { DiffViewMode } from "./useDiffViewMode"

export function DiffViewToggle({
  mode,
  setMode,
}: {
  mode: DiffViewMode
  setMode: (mode: DiffViewMode) => void
}) {
  const baseClass =
    "inline-flex items-center justify-center rounded-sm transition-colors p-1.5"
  const activeClass = "bg-background text-foreground shadow-sm"
  const inactiveClass = "text-muted-foreground hover:text-foreground"

  return (
    <div
      className="inline-flex items-center rounded-md border bg-muted p-0.5"
      tabIndex={-1}
    >
      <button
        onClick={() => setMode("unified")}
        tabIndex={-1}
        className={cn(
          baseClass,
          mode == "unified" ? activeClass : inactiveClass,
        )}
        title="Unified view"
      >
        <Rows3 className={"w-4 h-4"} />
      </button>
      <button
        onClick={() => setMode("split")}
        tabIndex={-1}
        className={cn(baseClass, mode == "split" ? activeClass : inactiveClass)}
        title="Split view"
      >
        <Columns2 className="w-4 h-4" />
      </button>
    </div>
  )
}
