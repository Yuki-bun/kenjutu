import { useState } from "react"

export type DiffViewMode = "unified" | "split"

const DIFF_VIEW_MODE_KEY = "revue-diff-view-mode"

export function useDiffViewMode() {
  const [diffViewMode, _setDiffViewMode] = useState<DiffViewMode>(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(DIFF_VIEW_MODE_KEY)
      if (stored === "unified" || stored === "split") {
        return stored
      }
    }
    return "unified"
  })

  const setDiffViewMode = (diffViewMode: DiffViewMode) => {
    _setDiffViewMode(diffViewMode)
    localStorage.setItem(DIFF_VIEW_MODE_KEY, diffViewMode)
  }

  const toggleDiffViewMode = () => {
    setDiffViewMode(diffViewMode === "unified" ? "split" : "unified")
  }

  return { diffViewMode, setDiffViewMode, toggleDiffViewMode }
}
