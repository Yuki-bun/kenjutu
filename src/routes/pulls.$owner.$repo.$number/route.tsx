import { useQuery } from "@tanstack/react-query"
import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { zodValidator } from "@tanstack/zod-adapter"
import { ExternalLink } from "lucide-react"
import { z } from "zod"

import { ErrorDisplay } from "@/components/error"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { useGithub } from "@/context/GithubContext"
import { useTab } from "@/hooks/useTab"
import { queryKeys } from "@/lib/queryKeys"
import { getLocalPath } from "@/lib/repos"

import { FilesTab } from "./-components/FilesTab"
import { OverviewTab } from "./-components/OverviewTab"
import { usePullRequest } from "./-hooks/usePullRequest"

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

  // Fetch local repo path from Tauri Store
  const { data: localDir } = useQuery({
    queryKey: queryKeys.localRepoPath(repoId),
    queryFn: () => getLocalPath(repoId),
  })

  const { data, error, isLoading, refetch } = usePullRequest(
    localDir ?? null,
    owner,
    repo,
    Number(number),
  )

  useTab(data ? `PR #${number}: ${data.title}` : `PR #${number} ${repo}`)

  // Full-width loading/error states before rendering layout
  if (isLoading) {
    return (
      <main className="h-full w-full p-4">
        <p className="text-muted-foreground">Loading pull request...</p>
      </main>
    )
  }

  if (error) {
    return (
      <main className="h-full w-full p-4">
        {error instanceof Error ? (
          error.message
        ) : (
          <ErrorDisplay error={error} />
        )}
      </main>
    )
  }

  const handleTabChange = (newTab: string) => {
    navigate({
      to: "/pulls/$owner/$repo/$number",
      params: { owner, repo, number },
      search: { repoId, tab: newTab },
    })
  }

  return (
    <main className="flex flex-col h-full w-full">
      {/* Fixed PR Header */}
      <div className="border-b px-6 py-4 shrink-0">
        <div className="flex gap-x-1">
          <h1 className="text-xl font-semibold">
            {data ? data.title : `Pull Request #${number}`}
          </h1>
          {data?.html_url && (
            <a href={data?.html_url} rel="noopener noreferrer" target="_blank">
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

      {/* Tabs */}
      <Tabs
        value={tab}
        onValueChange={handleTabChange}
        className="flex flex-col flex-1 overflow-hidden"
      >
        <div className="w-full border-b bg-transparent px-6 h-12 flex items-center justify-between">
          <TabsList className="rounded-none bg-transparent p-0 h-auto">
            <TabsTrigger value="overview">Overview</TabsTrigger>
            <TabsTrigger value="files">Files</TabsTrigger>
          </TabsList>
          <Button onClick={() => refetch()} variant="ghost" size="sm">
            Reload
          </Button>
        </div>

        <TabsContent
          value="overview"
          className="flex-1  mih-0 overflow-y-scroll mt-0 data-[state=inactive]:hidden"
        >
          <OverviewTab
            localDir={localDir ?? null}
            owner={owner}
            repo={repo}
            number={Number(number)}
            isAuthenticated={isAuthenticated}
          />
        </TabsContent>

        <TabsContent
          value="files"
          className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden"
        >
          <FilesTab
            localDir={localDir ?? null}
            owner={owner}
            repo={repo}
            prNumber={Number(number)}
          />
        </TabsContent>
      </Tabs>
    </main>
  )
}
