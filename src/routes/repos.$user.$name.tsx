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


export const Route = createFileRoute('/repos/$user/$name')({
  component: RouteComponent,
})

function RouteComponent() {
  const { user, name } = Route.useParams()

  const { data: repoData, error: repoError, refetch: refetchRepo } = useFailableQuery({
    queryKey: ["repo", { user, name }],
    queryFn: () => commands.getRepoById(user, name)
  })

  const { mutate } = useRpcMutation({
    mutationFn: (dir: string) => commands.setLocalRepo(user, name, dir),
    onSuccess: () => {
      refetchRepo()
    },
    onError: (err) => {
      toast(`failed to set local repository directory: ${err}`,
        { position: 'top-center', closeButton: true }
      )
    }
  })

  const { data: prData, error: prError, refetch } = useFailableQuery({
    queryKey: ["pullRequests", user, name],
    queryFn: () => commands.getPullRequests(user, name)
  })

  const handleSelectLocalRepo = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: `Select local repository for ${user}/${name}`,
    });

    if (selected && typeof selected === 'string') {
      mutate(selected);
    }
  };

  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>Pull Requests: {user}/{name}</CardTitle>
          <CardDescription>
            {repoData?.description}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {repoError && (
            <Alert variant="destructive" className="mb-4">
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>Error loading repository: {repoError}</AlertDescription>
            </Alert>
          )}
          {!repoData && !repoError && <p className="mb-4">Loading repository data...</p>}

          {repoData && (
            <div className="mb-4">
              <p className="flex items-center gap-2">
                Local repository: {repoData.local_repo ? repoData.local_repo : "Not Set"}
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
                  <TableHead>ID</TableHead>
                  <TableHead>Title</TableHead>
                  <TableHead>Author</TableHead>
                  <TableHead>GitHub URL</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {prData.map((pr) => (
                  <TableRow key={pr.id}>
                    <TableCell>{pr.id}</TableCell>
                    <TableCell>
                      <Link
                        to="/repos/$user/$name"
                        params={{ user, name }}
                        className="underline"
                      >
                        {pr.title ?? `PR #${pr.id}`}
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
                      <a href={pr.github_url ?? undefined} target="_blank" rel="noopener noreferrer" className="underline">
                        {pr.github_url}
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

          {prError && (
            <Alert variant="destructive" className="mt-4">
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>{prError}</AlertDescription>
            </Alert>
          )}
        </CardContent>
        <CardFooter>
          {/* Optional footer content */}
        </CardFooter>
      </Card>
    </main>
  )
}
