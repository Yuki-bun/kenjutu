import { useQuery, useQueryClient } from "@tanstack/react-query"
import { createFileRoute, Link } from "@tanstack/react-router"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"

import { commands } from "@/bindings"
import { getErrorMessage } from "@/components/error"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { useGithub } from "@/context/GithubContext"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { getLocalPath, setLocalPath } from "@/lib/repos"

import { useJjStatus } from "./-useJjStatus"
import { PullRequests, usePullRequests } from "./-usePullRequests"
import { useRepository } from "./-useRepository"

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
      toast(
        `Failed to set local repository directory: ${getErrorMessage(err)}`,
        {
          position: "top-center",
          closeButton: true,
        },
      )
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
          Local repository:{" "}
          {localRepoPath ? (
            <Link
              to={"/localRepo/$dir"}
              params={{ dir: localRepoPath }}
              disabled={!isJjRepo}
              className="underline"
            >
              {localRepoPath}
            </Link>
          ) : (
            "Not Set"
          )}
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
      <PullRequestsContent
        isAuthenticated={isAuthenticated}
        prLoading={prLoading}
        prData={prData ?? []}
        prError={prError}
        refetch={refetch}
        owner={owner}
        repo={repo}
        repoId={id}
      />
    </main>
  )
}

// Extracted PR content for reuse in both tabbed and non-tabbed views
type PullRequestsContentProps = {
  isAuthenticated: boolean
  prLoading: boolean
  prData: PullRequests
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
                    search={{ repoId, tab: "overview" }}
                    className="underline"
                  >
                    {pr.title ?? `PR #${pr.number}`}
                  </Link>
                </TableCell>
                <TableCell>
                  {pr.user ? (
                    <div className="flex items-center gap-2">
                      <div className="w-8 h-8 shrink-0 overflow-hidden rounded-full">
                        <img
                          src={pr.user.avatar_url}
                          alt={pr.user.login}
                          className="w-full h-full object-cover"
                        />
                      </div>
                      <span>{pr.user.name ?? pr.user.login}</span>
                    </div>
                  ) : (
                    <span className="italic text-gray-500">Unknown</span>
                  )}
                </TableCell>
                <TableCell>
                  <a
                    href={pr.html_url ?? undefined}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline"
                  >
                    {pr.html_url}
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
