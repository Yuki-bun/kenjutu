import { useLocation, useNavigate } from "@tanstack/react-router"
import { X } from "lucide-react"
import { useHotkeys } from "react-hotkeys-hook"

import { useTabs } from "@/context/TabsContext"
import { cn } from "@/lib/utils"

export function AppTabs() {
  const { tabs, closeTab } = useTabs()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  useHotkeys(
    "g,t",
    (e) => {
      e.preventDefault()
      if (tabs.length <= 1) return
      const currentIndex = tabs.findIndex((t) => t.path === pathname)
      const nextIndex = (currentIndex + 1) % tabs.length
      const nextTab = tabs[nextIndex]
      if (nextTab) {
        navigate({ to: nextTab.path, search: nextTab.search })
      }
    },
    { enableOnFormTags: true },
    [tabs, pathname, navigate],
  )

  useHotkeys(
    "g,shift+t",
    (e) => {
      e.preventDefault()
      if (tabs.length <= 1) return
      const currentIndex = tabs.findIndex((t) => t.path === pathname)
      const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length
      const prevTab = tabs[prevIndex]
      if (prevTab) {
        navigate({ to: prevTab.path, search: prevTab.search })
      }
    },
    { enableOnFormTags: true },
    [tabs, pathname, navigate],
  )

  if (tabs.length <= 1 && pathname !== "/") return null

  return (
    <div className="flex w-full shrink-0 items-center border-b bg-muted/40 px-2 overflow-x-auto [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden">
      {tabs.map((tab) => {
        const isActive = tab.path === pathname
        return (
          <div
            key={tab.path}
            onClick={() => navigate({ to: tab.path, search: tab.search })}
            className={cn(
              "group relative flex min-w-[150px] max-w-60 items-center justify-between gap-2 border-r px-4 py-2 text-sm transition-colors hover:bg-muted cursor-pointer select-none",
              isActive && "bg-background font-medium",
              !isActive && "text-muted-foreground",
            )}
          >
            <span className="truncate">{tab.title}</span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                closeTab(tab.path)
              }}
              className={cn(
                "rounded-sm opacity-0 ring-offset-background transition-opacity hover:bg-accent hover:text-accent-foreground focus:ring-2 focus:ring-ring focus:outline-none group-hover:opacity-100",
                isActive && "opacity-100",
              )}
            >
              <X className="h-3 w-3" />
              <span className="sr-only">Close tab</span>
            </button>
            {isActive && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
            )}
          </div>
        )
      })}
    </div>
  )
}
