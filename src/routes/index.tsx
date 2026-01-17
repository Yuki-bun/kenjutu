import { createFileRoute, Link } from "@tanstack/react-router"
import { useRepositories } from "@/hooks/useRepositories"
import { useGithub } from "@/context/GithubContext"
import { Button } from "@/components/ui/button"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
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
export const Route = createFileRoute("/")({
  component: RouteComponent,
})

function RouteComponent() {
  const { isAuthenticated } = useGithub()
  const { data, error, refetch, isLoading } = useRepositories()

  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>Welcome to PR Manager</CardTitle>
          <CardDescription>
            Manage your GitHub Pull Requests with ease.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {!isAuthenticated && (
            <Alert className="mb-4">
              <AlertTitle>Not Authenticated</AlertTitle>
              <AlertDescription>
                Please sign in with GitHub to view your repositories.
              </AlertDescription>
            </Alert>
          )}

          {isAuthenticated && (
            <>
              <div className="flex justify-end mb-4">
                <Button onClick={() => refetch()}>reload</Button>
              </div>

              {isLoading && <p>Loading repositories...</p>}

              {error && (
                <Alert variant="destructive" className="mb-4">
                  <AlertTitle>Error</AlertTitle>
                  <AlertDescription>
                    {error instanceof Error
                      ? error.message
                      : "Failed to load repositories"}
                  </AlertDescription>
                </Alert>
              )}

              {data && (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>owner</TableHead>
                      <TableHead>name</TableHead>
                      <TableHead>github url</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.map((repo) => (
                      <TableRow key={repo.id}>
                        <TableCell>{repo.ownerName}</TableCell>
                        <TableCell>
                          <Link
                            to="/repos/$owner/$repo"
                            params={{ owner: repo.ownerName, repo: repo.name }}
                            search={{ id: repo.id }}
                            className="underline"
                          >
                            {repo.name}
                          </Link>
                        </TableCell>
                        <TableCell>
                          <a
                            href={repo.htmlUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="underline"
                          >
                            {repo.htmlUrl}
                          </a>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </>
          )}
        </CardContent>
        <CardFooter>{/* Optional footer content */}</CardFooter>
      </Card>
    </main>
  )
}
