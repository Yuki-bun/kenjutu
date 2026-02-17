import "./index.css"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { createRouter, RouterProvider } from "@tanstack/react-router"
import { StrictMode } from "react"
import ReactDOM from "react-dom/client"
import { Toaster } from "sonner"

import { PaneManagerProvider } from "@/components/Pane/"
import { TooltipProvider } from "@/components/ui/tooltip"
import { GithubProvider } from "@/context/GithubContext"
import { ShaToChangeIdProvider } from "@/context/ShaToChangeIdContext"

import { routeTree } from "./routeTree.gen"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000,
    },
  },
})

const router = createRouter({ routeTree })

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router
  }
}

const rootElement = document.getElementById("root")!
if (!rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)
  root.render(
    <StrictMode>
      <GithubProvider>
        <QueryClientProvider client={queryClient}>
          <ShaToChangeIdProvider>
            <TooltipProvider>
              <PaneManagerProvider>
                <Toaster />
                <RouterProvider router={router} />
              </PaneManagerProvider>
            </TooltipProvider>
          </ShaToChangeIdProvider>
        </QueryClientProvider>
      </GithubProvider>
    </StrictMode>,
  )
}
