import { useHotkey } from "@tanstack/react-hotkeys"
import { Columns2, Rows3 } from "lucide-react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { cn } from "@/lib/utils"

import { useDiffContext } from "./CommitDiffSection"

export function Header() {
  const { files, diffViewMode, setDiffViewMode, toggleDiffViewMode } =
    useDiffContext()

  useHotkey("T", () => toggleDiffViewMode())

  const reviewedCount = files.filter(
    (f) => f.reviewStatus === "reviewed",
  ).length
  const progress = files.length > 0 ? (reviewedCount / files.length) * 100 : 0
  const revertedCount = files.filter(
    (f) => f.reviewStatus === "reviewedReverted",
  ).length

  if (files.length === 0) {
    return (
      <Alert>
        <AlertTitle>No Changes</AlertTitle>
        <AlertDescription>
          No file changes found in this commit.
        </AlertDescription>
      </Alert>
    )
  }

  const baseClass =
    "inline-flex items-center justify-center rounded-sm transition-colors p-1.5"
  const activeClass = "bg-background text-foreground shadow-sm"
  const inactiveClass = "text-muted-foreground hover:text-foreground"

  return (
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
            {reviewedCount}/{files.length}{" "}
            {revertedCount > 0 && `(${revertedCount} reverted)`}
          </span>
        </div>
      </div>
      <div
        className="inline-flex items-center rounded-md border bg-muted p-0.5"
        tabIndex={-1}
      >
        <button
          onClick={() => setDiffViewMode("unified")}
          tabIndex={-1}
          className={cn(
            baseClass,
            diffViewMode === "unified" ? activeClass : inactiveClass,
          )}
          title="Unified view"
        >
          <Rows3 className="w-4 h-4" />
        </button>
        <button
          onClick={() => setDiffViewMode("split")}
          tabIndex={-1}
          className={cn(
            baseClass,
            diffViewMode === "split" ? activeClass : inactiveClass,
          )}
          title="Split view"
        >
          <Columns2 className="w-4 h-4" />
        </button>
      </div>
    </div>
  )
}
