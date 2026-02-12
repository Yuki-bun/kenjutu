import { createRootRoute, Outlet } from "@tanstack/react-router"
import * as React from "react"

import { AppHeader } from "@/components/AppHeader"
import { AppTabs } from "@/components/AppTabs"
import { TabsProvider } from "@/context/TabsContext"

export const Route = createRootRoute({
  component: RootComponent,
})

function RootComponent() {
  return (
    <TabsProvider>
      <div className="flex flex-col h-screen">
        <AppHeader />
        <AppTabs />
        <div className="grow overflow-hidden flex flex-col">
          <Outlet />
        </div>
      </div>
    </TabsProvider>
  )
}
