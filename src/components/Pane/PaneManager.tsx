import { createContext, useContext, useRef } from "react"

export type PaneManagerContextType = {
  registerPane: (id: string, paneEntry: PaneEntry) => void
  unregisterPane: (id: string) => void
  focusPane: (id: string) => void
  focusPaneItem: (paneKey: string, itemKey: string) => void
  softFocusPaneItem: (paneKey: string, itemKey: string) => void
}

type PaneEntry = {
  container: HTMLElement
  onFocus: () => void
  onSoftFocusItem: (itemKey: string) => void
  onFocusItem: (itemKey: string) => void
}

const PaneManagerCOntext = createContext<PaneManagerContextType | null>(null)

export function usePaneManager() {
  const context = useContext(PaneManagerCOntext)
  if (!context) {
    throw new Error("usePaneManager must be used within a PaneManagerProvider")
  }
  return context
}

export function PaneManagerProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const panes = useRef<Map<string, PaneEntry>>(new Map())

  const registerPane = (id: string, paneEntry: PaneEntry) => {
    panes.current.set(id, paneEntry)
  }

  const unregisterPane = (id: string) => {
    panes.current.delete(id)
  }

  const focusPane = (id: string) => {
    const pane = panes.current.get(id)
    if (!pane) {
      console.warn(`Pane with id ${id} not found`)
      return
    }
    pane.onFocus()
  }

  const softFocusPaneItem = (paneKey: string, itemKey: string) => {
    const pane = panes.current.get(paneKey)
    if (!pane) {
      console.warn(`Pane with id ${paneKey} not found`)
      return
    }
    pane.onSoftFocusItem(itemKey)
  }

  const focusPaneItem = (paneKey: string, itemKey: string) => {
    const pane = panes.current.get(paneKey)
    if (!pane) {
      console.warn(`Pane with id ${paneKey} not found`)
      return
    }
    pane.onFocusItem(itemKey)
  }

  return (
    <PaneManagerCOntext.Provider
      value={{
        registerPane,
        unregisterPane,
        focusPane,
        softFocusPaneItem,
        focusPaneItem,
      }}
    >
      {children}
    </PaneManagerCOntext.Provider>
  )
}

export const PANEL_KEYS = {
  diffVew: "diff-view",
  fileTree: "file-tree",
  prCommitList: "pr-commit-list",
  commitGraph: "commit-graph",
} as const
