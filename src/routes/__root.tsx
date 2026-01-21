import * as React from "react"
import { Outlet, createRootRoute } from "@tanstack/react-router"
import { AppHeader } from "@/components/AppHeader"

export const Route = createRootRoute({
  component: RootComponent,
})

function RootComponent() {
  return (
    <React.Fragment>
      <div className="flex flex-col h-screen">
        <AppHeader />
        <div className="grow overflow-hidden">
          <Outlet />
        </div>
      </div>
    </React.Fragment>
  )
}
