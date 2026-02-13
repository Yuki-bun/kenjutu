import { Link, useNavigate } from "@tanstack/react-router"
import { Github } from "lucide-react"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { commands } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog"
import { useGithub } from "@/context/GithubContext"
import { useTabs } from "@/context/TabsContext"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

export function AppHeader() {
  const { isAuthenticated } = useGithub()
  const [isOpen, setIsOpen] = useState(false)
  const { tabs } = useTabs()
  const navigate = useNavigate()

  const authMutation = useRpcMutation({
    mutationFn: () => commands.authGithub(),
  })

  const isNotAuthenticated = !isAuthenticated
  const isAuthenticating = authMutation.isPending

  useHotkeys("meta+p", () => {
    setIsOpen((prev) => !prev)
  })

  return (
    <header className="z-50 w-full shrink-0 border-b bg-zinc-950">
      <nav className="w-full flex h-14 items-center justify-between px-4">
        {/* @ts-expect-error index route "/" not in generated types */}
        <Link to={"/"}>
          <div className="font-semibold text-lg">Revue</div>
        </Link>
        <Dialog open={isOpen} onOpenChange={setIsOpen}>
          <DialogTrigger asChild>
            <button onClick={() => setIsOpen((prev) => !prev)}>
              <div className="bg-accent py-1.5 rounded-md px-3">
                Search pages....
              </div>
            </button>
          </DialogTrigger>
          <DialogContent>
            <Command className="max-w-sm rounded-lg border">
              <CommandInput placeholder="Type a command or search..." />
              <CommandList>
                <CommandEmpty>No results found.</CommandEmpty>
                {tabs.map((tab) => (
                  <CommandItem
                    onSelect={() => {
                      navigate({
                        to: tab.path,
                        search: tab.search,
                      })
                      setIsOpen(false)
                    }}
                    key={tab.path}
                  >
                    {tab.title}
                  </CommandItem>
                ))}
                <CommandItem
                  onSelect={() => {
                    // @ts-expect-error index route "/" not in generated types
                    navigate({ to: "/" })
                    setIsOpen(false)
                  }}
                >
                  Home
                </CommandItem>
              </CommandList>
            </Command>
          </DialogContent>
        </Dialog>

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
