import { useNavigate } from "@tanstack/react-router"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command"
import { useTabs } from "@/context/TabsContext"
import { useLocalRepos } from "@/hooks/useLocalRepos"
import { useRepositories } from "@/routes/index/-hooks/useRepositories"

export function AppCommands() {
  const { tabs } = useTabs()
  const navigate = useNavigate()
  const [isOpen, setIsOpen] = useState(false)
  useHotkeys("meta+p", () => {
    setIsOpen((prev) => !prev)
  })

  const { data: repositories } = useRepositories()
  const { data: localRepos } = useLocalRepos()

  return (
    <>
      <button onClick={() => setIsOpen((prev) => !prev)}>
        <div className="bg-accent py-1.5 rounded-md px-3">Search pages....</div>
      </button>
      <CommandDialog open={isOpen} onOpenChange={setIsOpen}>
        <Command>
          <CommandInput placeholder="Type a command or search..." />
          <CommandList>
            <CommandEmpty>No results found.</CommandEmpty>
            <CommandGroup heading="Pages">
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
            </CommandGroup>
            {localRepos && localRepos.length > 0 && (
              <>
                <CommandSeparator />
                <CommandGroup heading="Local Repositories">
                  {localRepos.map((dir) => (
                    <CommandItem
                      key={dir}
                      onSelect={() => {
                        navigate({
                          to: "/localRepo/$dir",
                          params: { dir },
                        })
                        setIsOpen(false)
                      }}
                    >
                      {dir.split("/").pop()}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </>
            )}
            {repositories && repositories.length > 0 && (
              <>
                <CommandSeparator />
                <CommandGroup heading="Repositories">
                  {repositories.map((repo) => (
                    <CommandItem
                      key={repo.id}
                      onSelect={() => {
                        navigate({
                          to: "/repos/$owner/$repo",
                          params: {
                            owner: repo.owner.login,
                            repo: repo.name,
                          },
                          search: { id: repo.node_id },
                        })
                        setIsOpen(false)
                      }}
                    >
                      {repo.owner.login}/{repo.name}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </>
            )}
          </CommandList>
        </Command>
      </CommandDialog>
    </>
  )
}
