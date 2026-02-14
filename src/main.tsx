import "./index.css"

import { TooltipProvider } from "@radix-ui/react-tooltip"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { createRouter, RouterProvider } from "@tanstack/react-router"
import { StrictMode } from "react"
import ReactDOM from "react-dom/client"
import { Toaster } from "sonner"

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
              <Toaster />
              <RouterProvider router={router} />
            </TooltipProvider>
          </ShaToChangeIdProvider>
        </QueryClientProvider>
      </GithubProvider>
    </StrictMode>,
  )
}
