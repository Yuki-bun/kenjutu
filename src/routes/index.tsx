import { createFileRoute, Link } from "@tanstack/react-router"

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
import { useRepositories } from "@/hooks/useRepositories"

export const Route = createFileRoute("/")({
  component: RouteComponent,
})

function RouteComponent() {
  const { isAuthenticated } = useGithub()
  const { data: repos, error, refetch, isLoading } = useRepositories()

  return (
    <main className="h-full w-full p-4">
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

          {repos && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>owner</TableHead>
                  <TableHead>name</TableHead>
                  <TableHead>github url</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {repos.map((repo) => (
                  <TableRow key={repo.id}>
                    <TableCell>{repo.owner.login}</TableCell>
                    <TableCell>
                      <Link
                        to="/repos/$owner/$repo"
                        params={{ owner: repo.owner.login, repo: repo.name }}
                        search={{ id: repo.node_id }}
                        className="underline"
                      >
                        {repo.name}
                      </Link>
                    </TableCell>
                    <TableCell>
                      <a
                        href={repo.html_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="underline"
                      >
                        {repo.html_url}
                      </a>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </>
      )}
    </main>
  )
}
