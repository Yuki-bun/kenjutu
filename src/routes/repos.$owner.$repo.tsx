import { useQuery, useQueryClient } from "@tanstack/react-query"
import { createFileRoute, Link } from "@tanstack/react-router"
import { commands } from "@/bindings"
import { usePullRequests } from "@/hooks/usePullRequests"
import { useRepository } from "@/hooks/useRepository"
import { useJjStatus } from "@/hooks/useJjStatus"
import { useGithub } from "@/context/GithubContext"
import { getLocalPath, setLocalPath } from "@/lib/repos"
import { open } from "@tauri-apps/plugin-dialog"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { toast } from "sonner"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { LocalChangesTab } from "@/components/LocalChangesTab"

export const Route = createFileRoute("/repos/$owner/$repo")({
  component: RouteComponent,
  validateSearch: (params) => {
    const id = params.id
    if (typeof id !== "string") {
      throw new Error("Please pass node_id")
    }
    return { id }
  },
})

function RouteComponent() {
  const { owner, repo } = Route.useParams()
  const { id } = Route.useSearch()
  const { isAuthenticated } = useGithub()
  const queryClient = useQueryClient()

  const { data: repoData, error: repoError } = useRepository(owner, repo)

  const { data: localRepoPath, refetch: refetchLocalPath } = useQuery({
    queryKey: ["localRepoPath", id],
    queryFn: () => getLocalPath(id),
  })

  const setLocalRepoMutation = useRpcMutation({
    mutationFn: async (dir: string) => {
      const result = await commands.validateGitRepo(dir)
      if (result.status === "ok") {
        await setLocalPath(id, dir)
      }
      return result
    },
    onSuccess: () => {
      refetchLocalPath()
      queryClient.invalidateQueries({ queryKey: ["localRepoPath", id] })
    },
    onError: (err) => {
      let message = "Unknown Error"
      if (err.type === "BadInput") {
        message = err.description
      }
      toast(`Failed to set local repository directory: ${message}`, {
        position: "top-center",
        closeButton: true,
      })
    },
  })

  const {
    data: prData,
    error: prError,
    refetch,
    isLoading: prLoading,
  } = usePullRequests(owner, repo)

  // Check if this is a jj repository
  const { data: jjStatus } = useJjStatus(localRepoPath ?? undefined)
  const isJjRepo = jjStatus?.isJjRepo ?? false

  const handleSelectLocalRepo = async () => {
    const repoName = `${owner}/${repo}`
    const selected = await open({
      directory: true,
      multiple: false,
      title: `Select local repository for ${repoName}`,
    })

    if (selected && typeof selected === "string") {
      setLocalRepoMutation.mutate(selected)
    }
  }

  return (
    <main className="h-full w-full p-4 flex flex-col overflow-hidden">
      <div className="mb-6 shrink-0">
        <h1 className="text-2xl font-semibold">
          Pull Requests: {owner}/{repo}
        </h1>
        {repoData?.description && (
          <p className="text-muted-foreground">{repoData.description}</p>
        )}
      </div>

      {repoError && (
        <Alert variant="destructive" className="mb-4 shrink-0">
          <AlertTitle>Error</AlertTitle>
          <AlertDescription>
            {repoError instanceof Error
              ? repoError.message
              : "Failed to load repository"}
          </AlertDescription>
        </Alert>
      )}

      <div className="mb-4 shrink-0">
        <p className="flex items-center gap-2">
          Local repository: {localRepoPath ? localRepoPath : "Not Set"}
          <Button
            onClick={handleSelectLocalRepo}
            variant="outline"
            size="sm"
            disabled={setLocalRepoMutation.isPending}
          >
            {setLocalRepoMutation.isPending
              ? "Setting..."
              : "Select Local Repository"}
          </Button>
        </p>
      </div>

      {!isAuthenticated && (
        <Alert className="mb-4 shrink-0">
          <AlertTitle>Not Authenticated</AlertTitle>
          <AlertDescription>
            Please sign in with GitHub to view pull requests.
          </AlertDescription>
        </Alert>
      )}

      <Tabs
        defaultValue="pull-requests"
        className="flex flex-col flex-1 overflow-hidden"
      >
        <TabsList className="shrink-0">
          <TabsTrigger value="pull-requests">Pull Requests</TabsTrigger>
          <TabsTrigger
            disabled={!localRepoPath || !isJjRepo}
            value="local-changes"
          >
            Local Changes
          </TabsTrigger>
        </TabsList>
        <TabsContent value="pull-requests" className="flex-1 overflow-auto">
          <PullRequestsContent
            isAuthenticated={isAuthenticated}
            prLoading={prLoading}
            prData={prData}
            prError={prError}
            refetch={refetch}
            owner={owner}
            repo={repo}
            repoId={id}
          />
        </TabsContent>
        {!!localRepoPath && isJjRepo && (
          <TabsContent value="local-changes" className="flex-1 overflow-hidden">
            <LocalChangesTab localDir={localRepoPath} />
          </TabsContent>
        )}
      </Tabs>
    </main>
  )
}

// Extracted PR content for reuse in both tabbed and non-tabbed views
type PullRequestsContentProps = {
  isAuthenticated: boolean
  prLoading: boolean
  prData: ReturnType<typeof usePullRequests>["data"]
  prError: ReturnType<typeof usePullRequests>["error"]
  refetch: () => void
  owner: string
  repo: string
  repoId: string
}

function PullRequestsContent({
  isAuthenticated,
  prLoading,
  prData,
  prError,
  refetch,
  owner,
  repo,
  repoId,
}: PullRequestsContentProps) {
  return (
    <>
      {isAuthenticated && (
        <div className="flex justify-end mb-4 mt-4">
          <Button onClick={() => refetch()} variant="outline">
            reload PRs
          </Button>
        </div>
      )}

      {isAuthenticated && prLoading && <p>Loading pull requests...</p>}

      {prData && prData.length > 0 && (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Number #</TableHead>
              <TableHead>Title</TableHead>
              <TableHead>Author</TableHead>
              <TableHead>GitHub URL</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {prData.map((pr) => (
              <TableRow key={pr.id}>
                <TableCell>{pr.number}</TableCell>
                <TableCell>
                  <Link
                    to="/pulls/$owner/$repo/$number"
                    params={{
                      owner,
                      repo,
                      number: String(pr.number),
                    }}
                    search={{ repoId }}
                    className="underline"
                  >
                    {pr.title ?? `PR #${pr.number}`}
                  </Link>
                </TableCell>
                <TableCell>
                  {pr.author ? (
                    <div className="flex items-center gap-2">
                      <div className="w-8 h-8 shrink-0 overflow-hidden rounded-full">
                        <img
                          src={pr.author.avatar_url}
                          alt={pr.author.login}
                          className="w-full h-full object-cover"
                        />
                      </div>
                      <span>{pr.author.name ?? pr.author.login}</span>
                    </div>
                  ) : (
                    <span className="italic text-gray-500">Unknown</span>
                  )}
                </TableCell>
                <TableCell>
                  <a
                    href={pr.githubUrl ?? undefined}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline"
                  >
                    {pr.githubUrl}
                  </a>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      {prData && prData.length === 0 && (
        <Alert className="mt-4">
          <AlertTitle>No Pull Requests</AlertTitle>
          <AlertDescription>
            No pull requests found for this repository.
          </AlertDescription>
        </Alert>
      )}

      {prError && (
        <Alert variant="destructive" className="mt-4">
          <AlertTitle>Error</AlertTitle>
          <AlertDescription>
            {prError instanceof Error
              ? prError.message
              : "Failed to load pull requests"}
          </AlertDescription>
        </Alert>
      )}
    </>
  )
}
