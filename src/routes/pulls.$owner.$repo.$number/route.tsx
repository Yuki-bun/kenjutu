import { useIsFetching, useQuery, useQueryClient } from "@tanstack/react-query"
import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { zodValidator } from "@tanstack/zod-adapter"
import { ExternalLink } from "lucide-react"
import { useHotkeys } from "react-hotkeys-hook"
import { z } from "zod"

import { Button } from "@/components/ui/button"
import { CommandShortcut } from "@/components/ui/command"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { useGithub } from "@/context/GithubContext"
import { useTab } from "@/hooks/useTab"
import { queryKeys } from "@/lib/queryKeys"
import { getLocalPath } from "@/lib/repos"
import { cn } from "@/lib/utils"

import { FilesTab } from "./-components/FilesTab"
import { OverviewTab } from "./-components/OverviewTab"
import { usePullRequestDetails } from "./-hooks/usePullRequestDetails"

const routeScheme = z.object({
  repoId: z.string(),
  tab: z.string(),
})

export const Route = createFileRoute("/pulls/$owner/$repo/$number")({
  component: RouteComponent,
  validateSearch: zodValidator(routeScheme),
})

function RouteComponent() {
  const { number, owner, repo } = Route.useParams()
  const { repoId, tab } = Route.useSearch()
  const navigate = useNavigate()
  const { isAuthenticated } = useGithub()

  const { data: localDir } = useQuery({
    queryKey: queryKeys.localRepoPath(repoId),
    queryFn: () => getLocalPath(repoId),
  })

  const { data, error, isLoading } = usePullRequestDetails(
    owner,
    repo,
    Number(number),
  )

  useTab(data ? `PR #${number}: ${data.title}` : `PR #${number} ${repo}`)

  const handleTabChange = (newTab: string) => {
    navigate({
      to: "/pulls/$owner/$repo/$number",
      params: { owner, repo, number },
      search: { repoId, tab: newTab },
    })
  }

  const queryClient = useQueryClient()
  const handleReload = () =>
    queryClient.invalidateQueries({
      queryKey: queryKeys.pr(owner, repo, Number(number)),
    })

  const isFetching =
    useIsFetching({
      queryKey: queryKeys.pr(owner, repo, Number(number)),
    }) > 0

  useHotkeys("g>o", () => handleTabChange("overview"), [handleTabChange])
  useHotkeys("g>f", () => handleTabChange("files"), [handleTabChange])
  useHotkeys("g>r", handleReload, [handleReload])

  // Full-width loading/error states before rendering layout
  if (isLoading) {
    return (
      <main className="h-full w-full p-4">
        <p className="text-muted-foreground">Loading pull request...</p>
      </main>
    )
  }

  if (error) {
    return <main className="h-full w-full p-4">{error.message}</main>
  }

  return (
    <main className="flex flex-col h-full w-full">
      {/* Header with PR info and Tabs */}
      <div className="border-b px-6 py-4 shrink-0 flex items-end gap-4">
        <div className="flex-1">
          <div className="flex gap-x-1">
            <h1 className="text-xl font-semibold">
              {data ? data.title : `Pull Request #${number}`}
            </h1>
            {data?.html_url && (
              <a
                href={data?.html_url}
                rel="noopener noreferrer"
                target="_blank"
              >
                <ExternalLink />
              </a>
            )}
          </div>
          {data && (
            <p className="text-sm text-muted-foreground mt-1">
              {data.base.ref} &larr; {data.head.ref}
            </p>
          )}
        </div>

        <Tabs value={tab} onValueChange={handleTabChange}>
          <TabsList variant="line">
            <TabsTrigger value="overview" className="text-xl border-none">
              Overview
              <CommandShortcut>go</CommandShortcut>
            </TabsTrigger>
            <TabsTrigger value="files" className="text-xl border-none">
              Files
              <CommandShortcut>gf</CommandShortcut>
            </TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex-1" />

        <Button
          variant="secondary"
          onClick={handleReload}
          disabled={isFetching}
        >
          Reload
          <CommandShortcut className="bg-background">gr</CommandShortcut>
        </Button>
      </div>

      {/* Tab Content */}
      <div className="flex-1 min-h-0">
        <div
          className={cn("h-full overflow-y-auto", tab === "files" && "hidden")}
        >
          <OverviewTab
            localDir={localDir ?? null}
            owner={owner}
            repo={repo}
            number={Number(number)}
            isAuthenticated={isAuthenticated}
          />
        </div>
        <div
          className={cn(
            "h-full overflow-hidden",
            tab === "overview" && "hidden",
          )}
        >
          <FilesTab
            localDir={localDir ?? null}
            owner={owner}
            repo={repo}
            prNumber={Number(number)}
          />
        </div>
      </div>
    </main>
  )
}
