import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery, useRpcMutation } from "./../hooks/useRpcQuery"
import { open } from '@tauri-apps/plugin-dialog';
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
import { ErrorDisplay } from '@/components/error';


export const Route = createFileRoute('/repos/$nodeId')({
  component: RouteComponent,
})

function RouteComponent() {
  const { nodeId } = Route.useParams()

  const { data: repoData, error: repoError, refetch: refetchRepo } = useFailableQuery({
    queryKey: ["repo", nodeId],
    queryFn: () => commands.getRepoById(nodeId)
  })

  const { mutate } = useRpcMutation({
    mutationFn: (dir: string) => commands.setLocalRepo(nodeId, dir),
    onSuccess: () => {
      refetchRepo()
    },
    onError: (err) => {
      const meesage = err.type === 'BadInput' ? err.description : "Unknown Error"
      toast(`failed to set local repository directory: ${meesage}`,
        { position: 'top-center', closeButton: true }
      )
    }
  })

  const { data: prData, error: prError, refetch } = useFailableQuery({
    queryKey: ["pullRequests", nodeId],
    queryFn: () => commands.getPullRequests(nodeId)
  })

  const handleSelectLocalRepo = async () => {
    const repoName = repoData ? `${repoData.ownerName}/${repoData.name}` : 'repository'
    const selected = await open({
      directory: true,
      multiple: false,
      title: `Select local repository for ${repoName}`,
    });

    if (selected && typeof selected === 'string') {
      mutate(selected);
    }
  };

  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>
            Pull Requests: {repoData ? `${repoData.ownerName}/${repoData.name}` : 'Loading...'}
          </CardTitle>
          <CardDescription>
            {repoData?.description}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {repoError && <ErrorDisplay error={repoError} />}
          {!repoData && !repoError && <p className="mb-4">Loading repository data...</p>}

          {repoData && (
            <div className="mb-4">
              <p className="flex items-center gap-2">
                Local repository: {repoData.localRepo ? repoData.localRepo : "Not Set"}
                <Button onClick={handleSelectLocalRepo} variant="outline" size="sm">Select Local Repository</Button>
              </p>
            </div>
          )}

          <div className="flex justify-end mb-4">
            <Button onClick={() => refetch()} variant="outline">reload PRs</Button>
          </div>

          {!prData && !prError && <p>Loading pull requests...</p>}

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
                        to="/pulls/$nodeId/$number"
                        params={{
                          nodeId,
                          number: pr.number.toString()
                        }}
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
                      <a href={pr.githubUrl ?? undefined} target="_blank" rel="noopener noreferrer" className="underline">
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
              <AlertDescription>No pull requests found for this repository.</AlertDescription>
            </Alert>
          )}

          {prError && <ErrorDisplay error={prError} />}
        </CardContent>
        <CardFooter>
          {/* Optional footer content */}
        </CardFooter>
      </Card>
    </main>
  )
}
