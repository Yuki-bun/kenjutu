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
import { useRepositories } from "@/routes/index/-hooks/useRepositories"

export function AppCommands() {
  const { tabs } = useTabs()
  const navigate = useNavigate()
  const [isOpen, setIsOpen] = useState(false)
  useHotkeys("meta+p", () => {
    setIsOpen((prev) => !prev)
  })

  const { data } = useRepositories()

  return (
    <>
      <button onClick={() => setIsOpen((prev) => !prev)}>
        <div className="bg-accent py-1.5 rounded-md px-3">Search pages....</div>
      </button>
      <CommandDialog open={isOpen} onOpenChange={setIsOpen}>
        <Command>
          <CommandInput placeholder="Type a command or search..." />
          <CommandList className="scrollbar-none">
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
            {data && data.length > 0 && (
              <>
                <CommandSeparator />
                <CommandGroup heading="Repositories">
                  {data.map((repo) => (
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
