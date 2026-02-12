import { useLocation } from "@tanstack/react-router"
import { useEffect, useRef } from "react"

import { useTabs } from "@/context/TabsContext"

export function useTab(title: string) {
  const { registerTab } = useTabs()
  const location = useLocation()
  const initialPathname = useRef(location.pathname)

  useEffect(() => {
    if (location.pathname !== initialPathname.current) return
    const tab = {
      title,
      path: initialPathname.current,
      search: location.search,
    }
    registerTab(tab)
  }, [initialPathname, location, registerTab, title])
}
