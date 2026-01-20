import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { createFileRoute, Link } from "@tanstack/react-router"
import { commands } from "@/bindings"
import { usePullRequests } from "@/hooks/usePullRequests"
import { useRepository } from "@/hooks/useRepository"
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
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { toast } from "sonner"
import { useRpcMutation } from "@/hooks/useRpcQuery"

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
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>
            Pull Requests: {owner}/{repo}
          </CardTitle>
          <CardDescription>{repoData?.description}</CardDescription>
        </CardHeader>
        <CardContent>
          {repoError && (
            <Alert variant="destructive" className="mb-4">
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                {repoError instanceof Error
                  ? repoError.message
                  : "Failed to load repository"}
              </AlertDescription>
            </Alert>
          )}

          <div className="mb-4">
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
            <Alert className="mb-4">
              <AlertTitle>Not Authenticated</AlertTitle>
              <AlertDescription>
                Please sign in with GitHub to view pull requests.
              </AlertDescription>
            </Alert>
          )}

          {isAuthenticated && (
            <div className="flex justify-end mb-4">
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
                        search={{ repoId: id }}
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
        </CardContent>
        <CardFooter>{/* Optional footer content */}</CardFooter>
      </Card>
    </main>
  )
}
