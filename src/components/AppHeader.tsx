import { Github } from "lucide-react"
import { Button } from "@/components/ui/button"
import { commands } from "@/bindings"
import { useFailableQuery, useRpcMutation } from "@/hooks/useRpcQuery"
import { once } from "@tauri-apps/api/event"
import { toast } from "sonner"
import { useQueryClient } from "@tanstack/react-query"
import { cn } from "@/lib/utils"
import { Link } from "@tanstack/react-router"

export function AppHeader() {
  // TODO: implement proper authsate check
  // Check auth status using getRepositories query
  const testQuery = useFailableQuery({
    queryKey: ["repository"],
    queryFn: () => commands.getRepositories(),
    retry: false,
  })

  const queryClient = useQueryClient()

  const authMutation = useRpcMutation({
    mutationFn: () => commands.authGithub(),
    onSuccess: () => {
      once("authenticated", () => {
        toast("Successfully authenticated with GitHub")
        queryClient.invalidateQueries()
      })
    },
  })

  const isNotAuthenticated = testQuery.status == "error"
  const isAuthenticating = authMutation.isPending

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
      <nav className="container flex h-14 items-center justify-between px-4">
        <Link to={"/"}>
          <div className="font-semibold text-lg">PR Manager</div>
        </Link>

        {isNotAuthenticated && (
          <Button
            onClick={() => authMutation.mutate(undefined)}
            disabled={isAuthenticating}
            className={cn(
              "bg-[#24292f] text-white hover:bg-[#24292f]/90",
              "gap-2 shadow-sm",
            )}
          >
            <Github className="h-4 w-4" />
            <span className="hidden sm:inline">
              {isAuthenticating ? "Signing in..." : "Sign in with GitHub"}
            </span>
          </Button>
        )}
      </nav>
    </header>
  )
}
