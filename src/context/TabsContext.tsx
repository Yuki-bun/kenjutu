import { useLocation, useNavigate } from "@tanstack/react-router"
import React, {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from "react"

export type Tab = {
  title: string
  path: string
  search?: Record<string, unknown>
}

type TabsContextType = {
  tabs: Tab[]
  /** add tab entry for new path and update existing one for existing path */
  registerTab: (tab: Tab) => void
  closeTab: (id: string) => void
}

const TabsContext = createContext<TabsContextType | undefined>(undefined)

export function TabsProvider({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate()

  const [tabs, setTabs] = useState<Tab[]>([])
  const { pathname } = useLocation()
  const activeTab = tabs.find((tab) => tab.path === pathname)

  // Deferred registration avoids synchronous setState inside useEffect which
  // causes cascading renders and blocks React Compiler optimisation.
  // Multiple registrations within the same microtask are batched into one
  // setState call.
  const pendingRef = useRef<Tab[]>([])
  const flushScheduledRef = useRef(false)

  const registerTab = useCallback((tab: Tab) => {
    pendingRef.current.push(tab)
    if (!flushScheduledRef.current) {
      flushScheduledRef.current = true
      queueMicrotask(() => {
        const pending = pendingRef.current
        pendingRef.current = []
        flushScheduledRef.current = false
        setTabs((prev) => {
          let next = prev
          for (const t of pending) {
            if (next.some((existing) => existing.path === t.path)) {
              next = next.map((existing) =>
                existing.path === t.path ? t : existing,
              )
            } else {
              next = [...next, t]
            }
          }
          return next
        })
      })
    }
  }, [])

  const removeTab = useCallback((path: string) => {
    setTabs((prev) => prev.filter((t) => t.path !== path))
  }, [])

  const closeTab = useCallback(
    (path: string) => {
      if (activeTab?.path === path) {
        const tabIndex = tabs.findIndex((t) => t.path === path)
        const newTabs = tabs.filter((t) => t.path !== path)

        if (newTabs.length > 0) {
          const nextTab = newTabs[Math.min(tabIndex, newTabs.length - 1)]
          navigate({ to: nextTab.path, search: nextTab.search })
        } else {
          // @ts-expect-error index route "/" not in generated types
          navigate({ to: "/" })
        }
      }
      removeTab(path)
    },
    [activeTab?.path, removeTab, tabs, navigate],
  )

  const value = useMemo(
    () => ({
      tabs,
      registerTab,
      closeTab,
    }),
    [tabs, registerTab, closeTab],
  )

  return <TabsContext.Provider value={value}>{children}</TabsContext.Provider>
}

export function useTabs() {
  const context = useContext(TabsContext)
  if (!context) {
    throw new Error("useTabs must be used within a TabsProvider")
  }
  return context
}
