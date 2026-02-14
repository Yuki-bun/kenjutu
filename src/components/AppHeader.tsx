import { Link } from "@tanstack/react-router"
import { Github } from "lucide-react"

import { commands } from "@/bindings"
import { Button } from "@/components/ui/button"
import { useGithub } from "@/context/GithubContext"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

import { AppCommands } from "./AppCommands"

export function AppHeader() {
  const { isAuthenticated } = useGithub()

  const authMutation = useRpcMutation({
    mutationFn: () => commands.authGithub(),
  })

  const isNotAuthenticated = !isAuthenticated
  const isAuthenticating = authMutation.isPending

  return (
    <header className="z-50 w-full shrink-0 border-b">
      <nav className="w-full flex h-14 items-center justify-between px-4">
        {/* @ts-expect-error index route "/" not in generated types */}
        <Link to={"/"}>
          <div className="font-semibold text-lg">Revue</div>
        </Link>
        <AppCommands />

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
